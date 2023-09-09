use std::fs;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::Error;
use std::io::Read;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use tracing::{debug, error, info, warn};

use ffs::config::Config;
use ffs::config::Symlink;
use ffs::config::{ERROR_STATUS_CLI, ERROR_STATUS_FUSE};
use ffs::format;
use ffs::time_ns;
use format::json::Value as JsonValue;
use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;
use format::Format;
use format::Nodelike;
use format::Typ;

use ::xattr;
use regex::Regex;

pub struct SymlinkMapData {
    link: PathBuf,
    is_broken: bool,
}

pub struct Pack {
    // mapping of symlink to:
    // PathBuf of link destination
    // bool of whether symlink chain ends in a broken link
    pub symlinks: HashMap<PathBuf, SymlinkMapData>,
    depth: u32,
    regex: Regex,
}

impl Pack {
    pub fn new() -> Self {
        Self {
            symlinks: HashMap::new(),
            depth: 0,
            regex: Regex::new("^-?[0-9]+").unwrap(),
        }
    }

    pub fn pack<V>(&mut self, path: PathBuf, config: &Config) -> std::io::Result<Option<V>>
    where
        V: Nodelike + std::fmt::Display + Default,
    {
        // don't continue packing if max depth is reached
        if config
            .max_depth
            .is_some_and(|max_depth| self.depth > max_depth)
        {
            return Ok(None);
        }

        // get the type of data from xattr if it exists
        let mut path_type: Vec<u8> = Vec::new();

        if path.is_symlink() {
            match &config.symlink {
                Symlink::NoFollow => {
                    // early return because we want to ignore symlinks,
                    return Ok(None);
                }
                Symlink::Follow => {
                    let mut link_trail = Vec::new();
                    let mut link_follower = path.clone();
                    while link_follower.is_symlink() {
                        if link_trail.contains(&link_follower) {
                            error!("Symlink loop detected at {:?}.", link_follower);
                            std::process::exit(ERROR_STATUS_FUSE);
                        }
                        link_trail.push(link_follower.clone());

                        if path_type.is_empty() {
                            // get the xattr of the first symlink that has it defined.
                            // this has the effect of inheriting xattrs from links down the
                            // chain.
                            match xattr::get(&link_follower, "user.type") {
                                Ok(Some(xattr)) if config.allow_xattr => path_type = xattr,
                                Ok(_) | Err(_) => (),
                                // TODO(nad) 2023-08-07: maybe unnecessary to check for ._ as
                                // symlink?
                                // Err(_) => {
                                //     // Cannot call xattr::get on ._ file
                                //     warn!(
                                //         "._ files, like {:?}, prevent xattr calls. It will be encoded in base64.",
                                //         link_follower
                                //     );
                                //     path_type = b"bytes".to_vec()
                                // }
                            };
                        }

                        // add the link to the mapping to reduce future read_link calls for each
                        // symlink on the chain.
                        if !self.symlinks.contains_key(&link_follower) {
                            let link = link_follower.read_link()?;
                            self.symlinks.insert(
                                link_follower.clone(),
                                SymlinkMapData {
                                    link: if link.is_absolute() {
                                        link
                                    } else {
                                        link_follower.clone().parent().unwrap().join(link)
                                    },
                                    is_broken: false,
                                },
                            );
                        }
                        if self.symlinks[&link_follower].is_broken {
                            // .1 is a bool to tell if symlink is broken
                            // the symlink either is broken or links to a broken symlink.
                            // stop the traversal immediately and update mapping if possible
                            break;
                        }
                        link_follower = self.symlinks[&link_follower].link.clone();
                    }

                    if self.symlinks[link_trail.last().unwrap()].is_broken
                        || !link_follower.exists()
                    {
                        // the symlink is broken, so don't pack this file.
                        warn!(
                            "The symlink at the end of the chain starting from '{:?}' is broken.",
                            path
                        );
                        for link in link_trail {
                            let symlink_map_data = &self.symlinks[&link];
                            self.symlinks.insert(
                                link,
                                SymlinkMapData {
                                    link: symlink_map_data.link.to_path_buf(),
                                    is_broken: true,
                                },
                            );
                        }
                        return Ok(None);
                    }

                    // pack reached the actual destination
                    let canonicalized = link_follower.canonicalize()?;
                    if path.starts_with(&canonicalized) {
                        error!(
                            "The symlink {:?} points to some ancestor directory: {:?}, causing an infinite loop.",
                            path, canonicalized
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    if !config.allow_symlink_escape
                        && !canonicalized.starts_with(config.mount.as_ref().unwrap())
                    {
                        warn!("The symlink {:?} points to some file outside of the directory being packed. \
                              Specify --allow-symlink-escape to allow pack to follow this symlink.", path);
                        return Ok(None);
                    }
                }
            }
        }

        // if the xattr is still not set, either path is not a symlink or
        // none of the symlinks on the chain have an xattr. Use the actual file's xattr
        if path_type.is_empty() {
            let canonicalized = path.canonicalize()?;
            path_type = match xattr::get(&canonicalized, "user.type") {
                Ok(Some(xattr_type)) if config.allow_xattr => xattr_type,
                Ok(_) => b"auto".to_vec(),
                Err(_) => {
                    // Cannot call xattr::get on ._ file
                    warn!(
                        "._ files, like {:?}, prevent xattr calls. It will be encoded in base64.",
                        path
                    );
                    b"bytes".to_vec()
                }
            };
        }

        // convert detected xattr from Vec to str
        let mut path_type: &str = str::from_utf8(&path_type).unwrap();

        // resolve path type if it is 'auto'
        if path.is_dir() && (path_type == "auto" || path_type != "named" && path_type != "list") {
            if path_type != "auto" {
                warn!(
                    "Unknown directory type '{}'. Possible types are 'named' or 'list'. \
                    Resolving type automatically.",
                    path_type
                );
            }
            let all_files_begin_with_num = fs::read_dir(path.clone())?
                .map(|res| res.map(|e| e.path()))
                .map(|e| e.unwrap().file_name().unwrap().to_str().unwrap().to_owned())
                .all(|filename| self.regex.is_match(&filename));
            if all_files_begin_with_num {
                path_type = "list"
            } else {
                path_type = "named"
            };
        }

        info!("type of {:?} is {}", path, path_type);

        // return the value based on determined type
        match path_type {
            "named" => {
                let mut children = fs::read_dir(path.clone())?
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, Error>>()?;
                children.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));

                let mut entries = BTreeMap::new();

                for child in &children {
                    let child_name = child.file_name().unwrap().to_str().unwrap();
                    if config.ignored_file(child_name) {
                        warn!("skipping ignored file {:?}", child_name);
                        continue;
                    }
                    let name: String;
                    match xattr::get(&child, "user.original_name") {
                        Ok(Some(original_name)) if config.allow_xattr => {
                            let old_name = str::from_utf8(&original_name).unwrap();
                            if !config.valid_name(old_name) {
                                // original name must have been munged, so restore original
                                name = old_name.to_string();
                            } else {
                                // original name wasn't munged, keep the current name
                                // in case it was renamed
                                name = child_name.to_string();
                            }
                        }
                        Ok(_) | Err(_) => {
                            // use current name because either --no-xattr is set,
                            // xattr is None, or getting xattr on file (like ._ files) errors
                            name = child_name.to_string();
                        }
                    }
                    self.depth += 1;
                    let value = self.pack(child.clone(), &config)?;
                    self.depth -= 1;
                    if let Some(value) = value {
                        entries.insert(name, value);
                    }
                }

                Ok(Some(V::from_named_dir(entries, &config)))
            }
            "list" => {
                // TODO(nad) 2023-09-09 regex matching done twice
                // is this efficient?
                let mut numbers_filenames_paths = fs::read_dir(path.clone())?
                    .map(|res| res.map(|e| e.path()))
                    .map(|p| {
                        (
                            p.as_ref()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned(),
                            p.unwrap(),
                        )
                    })
                    .map(|(filename, p)| {
                        // store a triple (integer, file basename, full pathbuf)
                        // full pathbuf must be retained for symlink support.
                        (
                            match self.regex.find(&filename) {
                                Some(m) => filename[m.range()].parse::<i32>().unwrap(),
                                // use max i32 to give a default functionality for directories
                                // that are forced into being lists, which doesn't guarantee
                                // that filenames start with integers.
                                None => i32::MAX,
                            },
                            // filenames in a directory are guaranteed to be different, so it
                            // probably is the case that the PathBuf is never compared. Also,
                            // filename is much shorter than the entire path, so that also saves
                            // time.
                            filename,
                            p,
                        )
                    })
                    .collect::<Vec<_>>();
                numbers_filenames_paths.sort();

                info!("parsed numbers and filenames {:?}", numbers_filenames_paths);

                let mut entries = Vec::with_capacity(numbers_filenames_paths.len());
                for (_, filename, child) in numbers_filenames_paths {
                    if config.ignored_file(&filename) {
                        warn!("skipping ignored file {:?}", child);
                        continue;
                    }
                    self.depth += 1;
                    let value = self.pack(child, &config)?;
                    self.depth -= 1;
                    if let Some(value) = value {
                        entries.push(value);
                    }
                }

                Ok(Some(V::from_list_dir(entries, &config)))
            }
            typ => {
                if let Ok(t) = Typ::from_str(typ) {
                    let file = fs::File::open(&path).unwrap();
                    let mut reader = BufReader::new(&file);
                    let mut contents: Vec<u8> = Vec::new();
                    reader.read_to_end(&mut contents).unwrap();
                    match String::from_utf8(contents.clone()) {
                        Ok(mut contents) if t != Typ::Bytes => {
                            if config.add_newlines && contents.ends_with('\n') {
                                contents.truncate(contents.len() - 1);
                            }
                            Ok(Some(V::from_string(t, contents, &config)))
                        }
                        Ok(_) | Err(_) => Ok(Some(V::from_bytes(contents, &config))),
                    }
                } else {
                    error!(
                        "This error should never be called. Received undetected and unknown type '{}' for file '{}'",
                        typ,
                        path.display()
                    );
                    std::process::exit(ERROR_STATUS_FUSE);
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let config = Config::from_pack_args();
    debug!("received config: {:?}", config);

    let mount = match &config.mount {
        Some(mount) => mount,
        None => {
            error!("Cannot pack unspecified directory.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    let folder = PathBuf::from(mount);

    let writer = match config.output_writer() {
        Some(writer) => writer,
        None => return Ok(()),
    };

    let mut packer: Pack = Pack::new();

    match &config.output_format {
        Format::Json => {
            let v: JsonValue = time_ns!(
                "saving",
                packer.pack(folder, &config)?.unwrap(),
                config.timing
            );

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
        Format::Toml => {
            let v: TomlValue = time_ns!(
                "saving",
                packer.pack(folder, &config)?.unwrap(),
                config.timing
            );

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
        Format::Yaml => {
            let v: YamlValue = time_ns!(
                "saving",
                packer.pack(folder, &config)?.unwrap(),
                config.timing
            );

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
    }

    Ok(())
}
