use std::fs;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use clap::Arg;
use clap::ArgAction;
use clap::Command;
use clap::value_parser;
use nodelike::ParseFormatError;
use nodelike::config::Input;
use nodelike::config::Output;
use nodelike::config::POSSIBLE_FORMATS;
use tracing::debug;
use tracing::{error, warn};

use nodelike::Format;
use nodelike::Nodelike;
use nodelike::Typ;
use nodelike::config::Config;
use nodelike::config::Symlink;
use nodelike::config::{ERROR_STATUS_CLI, ERROR_STATUS_FUSE};
use nodelike::json::Value as JsonValue;
use nodelike::time_ns;
use nodelike::toml::Value as TomlValue;
use nodelike::yaml::Value as YamlValue;

use regex::Regex;

pub fn pack_cli() -> Command {
    nodelike::config::cli_base("pack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("pack directory")
        .arg(
            Arg::new("NOFOLLOW_SYMLINKS")
                .help("Never follow symbolic links. This is the default behaviour. `pack` will ignore all symbolic links.")
                .short('P')
                .overrides_with("FOLLOW_SYMLINKS")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("FOLLOW_SYMLINKS")
                .help("Follow all symlinks. For safety, you can also specify a --max-depth value.")
                .short('L')
                .overrides_with("NOFOLLOW_SYMLINKS")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("MAXDEPTH")
                .help("Maximum depth of filesystem traversal allowed for `pack`")
                .long("max-depth")
                .value_name("MAXDEPTH")
                .value_parser(value_parser!(u32))
        )
        .arg(
            Arg::new("ALLOW_SYMLINK_ESCAPE")
                .help("Allows pack to follow symlinks outside of the directory being packed.")
                .long("allow-symlink-escape")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("KEEPMACOSDOT")
                .help("Include ._* extended attribute/resource fork files on macOS")
                .long("keep-macos-xattr")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("OUTPUT")
                .help("Sets the output file for saving changes (defaults to stdout)")
                .long("output")
                .short('o')
                .value_name("OUTPUT")
        )
        .arg(
            Arg::new("NOOUTPUT")
                .help("Disables output of filesystem (normally on stdout)")
                .long("no-output")
                .overrides_with("OUTPUT")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("TARGET_FORMAT")
                .help("Specify the target format explicitly (by default, automatically inferred from filename extension)")
                .long("target")
                .short('t')
                .value_name("TARGET_FORMAT")
                .value_parser(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::new("PRETTY")
                .help("Pretty-print output (may increase size)")
                .long("pretty")
                .overrides_with("NOOUTPUT")
                .overrides_with("QUIET")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("INPUT")
                .help("The directory to be packed")
                .index(1),
        )
}

pub fn config_from_pack_args() -> Config {
    let (mut config, args) = Config::from_cli(pack_cli);

    // simple flags
    config.allow_symlink_escape = args.get_flag("ALLOW_SYMLINK_ESCAPE");
    config.keep_macos_xattr_file = args.get_flag("KEEPMACOSDOT");
    config.pretty = args.get_flag("PRETTY");

    config.symlink = if args.get_flag("FOLLOW_SYMLINKS") {
        Symlink::Follow
    } else {
        Symlink::NoFollow
    };

    config.max_depth = args.get_one::<u32>("MAXDEPTH").copied();

    // configure input
    config.input = match args.get_one::<String>("INPUT") {
        Some(input_source) => {
            let input_source = PathBuf::from(input_source);
            if !input_source.exists() {
                error!("Input file {} does not exist.", input_source.display());
                std::process::exit(ERROR_STATUS_FUSE);
            }
            Input::File(input_source)
        }
        None => {
            error!("The directory to pack must be specified.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    // set the mount from the input directory
    config.mount = match &config.input {
        Input::File(file) => Some(file.clone().canonicalize().unwrap()),
        _ => {
            error!("Input must be a file or directory.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    // configure output
    config.output = if let Some(output) = args.get_one::<String>("OUTPUT") {
        Output::File(PathBuf::from(output))
    } else if args.get_flag("NOOUTPUT") || args.get_flag("QUIET") {
        Output::Quiet
    } else {
        Output::Stdout
    };

    // try to autodetect the output format.
    //
    // first see if it's specified and parses okay.
    //
    // then see if we can pull it out of the extension (if specified)
    //
    // then give up and use the input format
    config.output_format = match args
        .get_one::<String>("TARGET_FORMAT")
        .ok_or(ParseFormatError::NoFormatProvided)
        .and_then(|s| s.parse::<Format>())
    {
        Ok(target_format) => target_format,
        Err(e) => {
            match e {
                ParseFormatError::NoSuchFormat(s) => {
                    warn!("Unrecognized format '{s}', inferring from input and output.")
                }
                ParseFormatError::NoFormatProvided => {
                    debug!("Inferring output format from input.")
                }
            };
            match args
                .get_one::<String>("OUTPUT")
                .and_then(|s| Path::new(s).extension())
                .and_then(|s| s.to_str())
            {
                Some(s) => match s.parse::<Format>() {
                    Ok(format) => format,
                    Err(_) => {
                        warn!(
                            "Unrecognized format {s}, defaulting to input format '{}'.",
                            config.input_format
                        );
                        config.input_format
                    }
                },
                None => config.input_format,
            }
        }
    };

    if config.pretty && !config.output_format.can_be_pretty() {
        warn!(
            "There is no pretty printing routine for {}.",
            config.output_format
        )
    }

    config
}

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

impl Default for Pack {
    fn default() -> Self {
        Self::new()
    }
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
                            error!("Symlink loop detected at {}.", link_follower.display());
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
                                //         "._ files, like {}, prevent xattr calls. It will be encoded in base64.",
                                //         link_follower.display()
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
                            "The symlink at the end of the chain starting from '{}' is broken.",
                            path.display()
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
                            "The symlink {} points to some ancestor directory: {}, causing an infinite loop.",
                            path.display(),
                            canonicalized.display(),
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    if !config.allow_symlink_escape
                        && !canonicalized.starts_with(config.mount.as_ref().unwrap())
                    {
                        warn!(
                            "The symlink {} points to some file outside of the directory being packed. \
                              Specify --allow-symlink-escape to allow pack to follow this symlink.",
                            path.display()
                        );
                        return Ok(None);
                    }
                }
            }
        }

        // if the xattr is still not set, either path is not a symlink or
        // none of the symlinks on the chain have an xattr. Use the actual file's xattr
        if path_type.is_empty() {
            let canonicalized = path.canonicalize()?;
            path_type = match xattr::get(canonicalized, "user.type") {
                Ok(Some(xattr_type)) if config.allow_xattr => xattr_type,
                Ok(_) => b"auto".to_vec(),
                Err(_) => {
                    // Cannot call xattr::get on ._ file
                    warn!(
                        "._ files, like {}, prevent xattr calls. It will be encoded in base64.",
                        path.display(),
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
                    "Unknown directory type '{path_type}'. Possible types are 'named' or 'list'. \
                    Resolving type automatically."
                );
            }
            let all_files_begin_with_num = fs::read_dir(path.clone())?
                .map(|res| res.map(|e| e.path()))
                .map(|e| e.unwrap().file_name().unwrap().to_str().unwrap().to_owned())
                .all(|filename| {
                    filename.chars().nth(0).unwrap().is_ascii_digit()
                        || filename.len() > 1
                            && filename.chars().nth(0).unwrap() == '-'
                            && filename.chars().nth(1).unwrap().is_ascii_digit()
                });
            if all_files_begin_with_num {
                path_type = "list"
            } else {
                path_type = "named"
            };
        }

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
                        warn!("skipping ignored file {}", child.display());
                        continue;
                    }
                    let name: String;
                    match xattr::get(child, "user.original_name") {
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
                    let value = self.pack(child.clone(), config)?;
                    self.depth -= 1;
                    if let Some(value) = value {
                        entries.insert(name, value);
                    }
                }

                Ok(Some(V::from_named_dir(entries, config)))
            }
            "list" => {
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

                let mut entries = Vec::with_capacity(numbers_filenames_paths.len());
                for (_, filename, child) in numbers_filenames_paths {
                    if config.ignored_file(&filename) {
                        warn!("skipping ignored file {}", child.display());
                        continue;
                    }
                    self.depth += 1;
                    let value = self.pack(child, config)?;
                    self.depth -= 1;
                    if let Some(value) = value {
                        entries.push(value);
                    }
                }

                Ok(Some(V::from_list_dir(entries, config)))
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
                            Ok(Some(V::from_string(t, contents, config)))
                        }
                        Ok(_) | Err(_) => Ok(Some(V::from_bytes(contents, config))),
                    }
                } else {
                    error!(
                        "Received undetected and unknown type '{typ}' for file '{}'",
                        path.display()
                    );
                    std::process::exit(ERROR_STATUS_FUSE);
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let config = config_from_pack_args();

    let mount = match &config.mount {
        Some(mount) => mount,
        None => {
            error!("You must specify a directory to pack.");
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
