use std::fs;
// use std::os::unix::fs::MetadataExt;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::Error;
use std::io::Read;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use tracing::{error, info, warn};

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

pub struct Pack {
    pub symlinks: HashMap<PathBuf, PathBuf>,
    depth: u32,
}

impl Pack {
    pub fn new() -> Self {
        Self {
            symlinks: HashMap::new(),
            depth: 0,
        }
    }

    pub fn pack<V>(&mut self, path: PathBuf, config: &Config) -> std::io::Result<Option<V>>
    where
        V: Nodelike + std::fmt::Display + Default,
    {
        if config
            .max_depth
            .is_some_and(|max_depth| self.depth > max_depth)
        {
            self.depth -= 1;
            return Ok(None);
        }

        // resolve symlink once and map the path to the linked file or
        // to itself if it is not a symlink
        if !self.symlinks.contains_key(&path) {
            if !path.is_symlink() {
                self.symlinks.insert(path.clone(), path.clone());
            } else {
                let link = path.read_link()?;
                if link.is_absolute() {
                    self.symlinks.insert(path.clone(), link);
                } else {
                    self.symlinks
                        .insert(path.clone(), path.clone().parent().unwrap().join(link));
                }
            }
        }

        // get the xattr of the file
        let mut path_type: Vec<u8> = Vec::new();
        // if it is a symlink
        if path.is_symlink() {
            match &config.symlink {
                // early return because we want to ignore this symlink,
                // but we still add it to self.symlinks above
                Symlink::NoFollow => {
                    self.depth -= 1;
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
                            // get the xattr of the first symlink that has it.
                            // this has the effect of inheriting xattrs from links down the
                            // chain.
                            match xattr::get(&link_follower, "user.type") {
                                Ok(Some(xattr)) if config.allow_xattr => path_type = xattr,
                                Ok(_) => (),
                                // TODO(nad) 2023-08-07: maybe unnecessary to check for ._ as
                                // symlink?
                                Err(_) => {
                                    // Cannot call xattr::get on ._ file
                                    warn!(
                                        "._ files, like {:?}, prevent xattr calls. It will be encoded in base64.",
                                        link_follower
                                    );
                                    path_type = b"bytes".to_vec()
                                }
                            };
                        }

                        // add the link to the mapping here if it wasn't already added above
                        if !self.symlinks.contains_key(&link_follower) {
                            let link = link_follower.read_link()?;
                            if link.is_absolute() {
                                self.symlinks.insert(link_follower.clone(), link);
                            } else {
                                self.symlinks.insert(
                                    link_follower.clone(),
                                    link_follower.clone().parent().unwrap().join(link),
                                );
                            }
                        }
                        link_follower = self.symlinks[&link_follower].clone();
                    }

                    if !link_follower.exists() {
                        error!("The symlink {:?} is broken.", path);
                        std::process::exit(ERROR_STATUS_FUSE);
                    }

                    // we reached the actual destination
                    let canonicalized = link_follower.canonicalize()?;
                    if path.starts_with(&canonicalized) {
                        error!(
                            "The symlink {:?} points to some ancestor directory: {:?}",
                            path, canonicalized
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    if !config.allow_symlink_escape
                        && !canonicalized.starts_with(config.mount.clone().unwrap())
                    {
                        warn!("The symlink {:?} points to some file outside of the directory being packed. \
                              Specify --allow-symlink-escape to allow pack to follow this symlink.", path);
                        self.depth -= 1;
                        return Ok(None);
                    }
                }
            }
        }

        // if the xattr isn't detected yet, either path is not a symlink or
        // none of the symlinks on the chain have an xattr.
        if path_type.is_empty() {
            let canonicalized = path.canonicalize()?;
            path_type = match xattr::get(&canonicalized, "user.type") {
                Ok(Some(xattr_type)) if config.allow_xattr => xattr_type,
                Ok(_) => b"detect".to_vec(),
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

        // resolve type if it is 'detect'
        if path_type == "detect" {
            if path.is_file() {
                path_type = "auto";
            } else if path.is_dir() {
                let try_parsing_to_int = fs::read_dir(path.clone())?
                    .map(|res| res.map(|e| e.path()))
                    .map(|e| {
                        e.unwrap()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .parse::<u32>()
                    })
                    .collect::<Result<Vec<_>, _>>();
                info!("parsed names or parse error: {:?}", try_parsing_to_int);
                match try_parsing_to_int {
                    Ok(mut parsed_ints) => {
                        parsed_ints.sort_unstable();
                        let mut i = 0;
                        for parsed_int in parsed_ints.clone() {
                            if parsed_int != i {
                                info!(
                                    "file {} is missing from the range of the number of files [0,{})",
                                    i, parsed_ints.len() as u32
                                );
                                path_type = "map";
                                break;
                            }
                            i += 1;
                        }
                        if i == parsed_ints.len() as u32 {
                            path_type = "list";
                        }
                    }
                    Err(_) => {
                        path_type = "named";
                    }
                }
            } else {
                error!("{:?} has unknown type and it is an unsupported file type (i.e. not file, directory).", path.display());
                std::process::exit(ERROR_STATUS_FUSE);
            }
        }

        info!("type of {:?} is {}", path, path_type);

        // return the value
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
                        Ok(Some(original_name)) => {
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
                            // use current name because xattr is None or getting xattr on files (like ._ files) errors
                            name = child_name.to_string();
                        }
                    }
                    self.depth += 1;
                    let value = self.pack(child.clone(), &config)?;
                    match value {
                        Some(value) => {
                            entries.insert(name, value);
                        }
                        None => continue,
                    }
                }

                Ok(Some(V::from_named_dir(entries, &config)))
            }
            "list" => {
                let mut children = fs::read_dir(path.clone())?
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, Error>>()?;
                children.sort_unstable_by(|a, b| {
                    a.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .cmp(&b.file_name().unwrap().to_str().unwrap())
                });

                let mut entries = Vec::with_capacity(children.len());

                for child in children {
                    let child_name = child.file_name().unwrap().to_str().unwrap();
                    if config.ignored_file(child_name) {
                        warn!("skipping ignored file {:?}", child_name);
                        continue;
                    }
                    self.depth += 1;
                    let value = self.pack(child, &config)?;
                    match value {
                        Some(value) => {
                            entries.push(value);
                        }
                        None => continue,
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
                            self.depth -= 1;
                            Ok(Some(V::from_string(t, contents, &config)))
                        }
                        Ok(_) | Err(_) => {
                            self.depth -= 1;
                            Ok(Some(V::from_bytes(contents, &config)))
                        }
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
    info!("received config: {:?}", config);

    let mount = match &config.mount {
        Some(mount) => mount,
        None => {
            error!("Cannot pack unspecified directory.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    let mut folder = PathBuf::from(mount);
    folder = folder.canonicalize()?;

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
