use clap::{Arg, ArgAction, Command};
use tracing::{debug, error, info, warn};

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use nodelike::config::{
    Config, ERROR_STATUS_CLI, ERROR_STATUS_FUSE, Input, Munge, POSSIBLE_FORMATS,
};
use nodelike::json::Value as JsonValue;
use nodelike::toml::Value as TomlValue;
use nodelike::yaml::Value as YamlValue;
use nodelike::{Format, Node, Nodelike, ParseFormatError, Typ};

pub fn unpack_cli() -> Command {
    nodelike::config::cli_base("unpack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("unpack structured data into a directory")
        .arg(
            Arg::new("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("TYPE")
                .help("Specify the format type explicitly (by default, automatically inferred from filename extension)")
                .long("type")
                .short('t')
                .value_name("TYPE")
                .value_parser(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::new("INTO")
                .help("Sets the directory in which to unpack the file; will be inferred when using a file, but must be specified when running on stdin")
                .long("into")
                .short('i')
                .value_name("INTO")
        )
        .arg(
            Arg::new("INPUT")
                .help("Sets the input file ('-' means STDIN)")
                .default_value("-")
                .index(1),
        )
}

pub fn config_from_unpack_args() -> Config {
    let (mut config, args) = Config::from_cli(unpack_cli);

    // simple flags
    config.pad_element_names = !args.get_flag("UNPADDED");

    // configure input
    config.input = match args.get_one::<String>("INPUT") {
        Some(input_source) => {
            if input_source == "-" {
                Input::Stdin
            } else {
                let input_source = PathBuf::from(input_source);
                if !input_source.exists() {
                    error!("Input file {} does not exist.", input_source.display());
                    std::process::exit(ERROR_STATUS_FUSE);
                }
                Input::File(input_source)
            }
        }
        None => Input::Stdin,
    };

    // infer and create mountpoint from filename as possible
    config.mount = match args.get_one::<String>("INTO") {
        Some(mount_point) => {
            match std::fs::create_dir(mount_point) {
                Ok(_) => Some(PathBuf::from(mount_point)),
                Err(_) => {
                    // if dir is empty then we can use it
                    let mount = PathBuf::from(mount_point);
                    if mount.read_dir().unwrap().next().is_none() {
                        // dir exists and is empty
                        Some(PathBuf::from(mount_point))
                    } else {
                        // dir exists but is not empty
                        error!("Directory `{mount_point}` already exists and is not empty.");
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                }
            }
        }
        None => {
            match &config.input {
                Input::Stdin => {
                    error!("You must specify a mount point when reading from stdin.");
                    std::process::exit(ERROR_STATUS_CLI);
                }
                Input::Empty => {
                    error!(
                        "--new is not an option for `unpack`, so the input should never be Empty and this error should never be seen."
                    );
                    std::process::exit(ERROR_STATUS_CLI);
                }
                Input::File(file) => {
                    // If the input is from a file foo.EXT, then try to make a directory foo.
                    let stem = file.file_stem().unwrap_or_else(|| {
                            error!("Couldn't infer the directory to unpack into from input '{}'. Use `--into DIRECTORY` to specify a directory.", file.display());
                            std::process::exit(ERROR_STATUS_FUSE);
                        });
                    let mount_dir = PathBuf::from(stem);
                    debug!("inferred mount_dir {}", mount_dir.display());

                    // If that file already exists, give up and tell the user about --mount.
                    if mount_dir.exists() {
                        error!(
                            "Inferred directory '{mount}' for input file '{file}', but '{mount}' already exists. Use `--into DIRECTORY` to specify a directory.",
                            mount = mount_dir.display(),
                            file = file.display()
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    // If the mountpoint can't be created, give up and tell the user about --mount.
                    if let Err(e) = std::fs::create_dir(&mount_dir) {
                        error!(
                            "Couldn't create directory '{}': {e}. Use `--into DIRECTORY` to specify a directory.",
                            mount_dir.display()
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }

                    // We did it!
                    Some(mount_dir)
                }
            }
        }
    };
    assert!(config.mount.is_some());

    // try to autodetect the input format.
    //
    // first see if it's specified and parses okay.
    //
    // then see if we can pull it out of the extension.
    //
    // then give up and use json
    config.input_format = match args
        .get_one::<String>("TYPE")
        .ok_or(ParseFormatError::NoFormatProvided)
        .and_then(|s| s.parse::<Format>())
    {
        Ok(source_format) => source_format,
        Err(e) => {
            match e {
                ParseFormatError::NoSuchFormat(s) => {
                    warn!("Unrecognized format '{s}', inferring from input.")
                }
                ParseFormatError::NoFormatProvided => {
                    debug!("Inferring format from input.")
                }
            };
            match &config.input {
                Input::Stdin => Format::Json,
                Input::Empty => Format::Json,
                Input::File(input_source) => match input_source
                    .extension()
                    .and_then(|s| s.to_str())
                    .ok_or(ParseFormatError::NoFormatProvided)
                    .and_then(|s| s.parse::<Format>())
                {
                    Ok(format) => format,
                    Err(_) => {
                        warn!(
                            "Unrecognized format {}, defaulting to JSON.",
                            input_source.display()
                        );
                        Format::Json
                    }
                },
            }
        }
    };

    config
}

fn unpack<V>(root: V, root_path: PathBuf, config: &Config) -> std::io::Result<()>
where
    V: Nodelike + std::fmt::Display + Default,
{
    let mut queue: VecDeque<(V, PathBuf, Option<String>)> = VecDeque::new();
    queue.push_back((root, root_path.clone(), None));

    while let Some((v, path, original_name)) = queue.pop_front() {
        match v.node(config) {
            Node::String(t, s) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(&path)?;

                // write `s` into that file
                f.write_all(s.as_bytes())?;

                // set metadata according to `t`
                if config.allow_xattr {
                    xattr::set(&path, "user.type", t.to_string().as_bytes())?;
                }
            }
            Node::Bytes(b) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(&path)?;

                // write `b` into that file
                f.write_all(b.as_slice())?;

                // set metadata to bytes
                if config.allow_xattr {
                    xattr::set(&path, "user.type", Typ::Bytes.to_string().as_bytes())?;
                }
            }
            Node::List(vs) => {
                // if not root path, make directory
                if path != root_path.clone() {
                    fs::create_dir(&path)?;
                }
                if config.allow_xattr {
                    xattr::set(&path, "user.type", "list".as_bytes())?;
                }

                // enqueue children with appropriate names
                let num_elts = vs.len() as f64;
                let width = num_elts.log10().ceil() as usize;

                for (i, child) in vs.into_iter().enumerate() {
                    // TODO(mmg) 2021-06-08 ability to add prefixes
                    let name = if config.pad_element_names {
                        format!("{i:0width$}")
                    } else {
                        format!("{i}")
                    };
                    let child_path = path.join(name);

                    queue.push_back((child, child_path, None));
                }
            }
            Node::Map(fvs) => {
                // if not root path, make directory
                if path != root_path.clone() {
                    fs::create_dir(&path)?;
                }
                if config.allow_xattr {
                    xattr::set(&path, "user.type", "named".as_bytes())?;
                }

                // enqueue children with appropriate names
                let mut child_names = std::collections::HashSet::new();
                for (field, child) in fvs.into_iter() {
                    let original = field.clone();

                    // munge name to be valid and unique
                    let name = if !config.valid_name(&original) {
                        match config.munge {
                            Munge::Rename => {
                                let mut nfield = config.normalize_name(field);

                                while child_names.contains(&nfield) {
                                    nfield.push('_');
                                }

                                nfield
                            }
                            Munge::Filter => {
                                // TODO(mmg) 2023-03-06 support logging
                                warn!("skipping '{field}'");
                                continue;
                            }
                        }
                    } else {
                        field
                    };
                    child_names.insert(name.clone());

                    let child_path = path.join(name);
                    queue.push_back((child, child_path, Some(original)));
                }
            }
        }

        if let Some(original_name) = original_name
            && config.allow_xattr
        {
            xattr::set(&path, "user.original_name", original_name.as_bytes())?;
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let config = config_from_unpack_args();

    let mount = match &config.mount {
        Some(mount) => mount.clone(),
        None => {
            error!("You must specify a directory to unpack.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };
    info!("mount: {mount:?}");

    let reader = match config.input_reader() {
        Some(reader) => reader,
        None => {
            error!("Input not specified");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    match &config.input_format {
        Format::Json => {
            let value = JsonValue::from_reader(reader);
            if value.is_dir() {
                unpack(value, mount.clone(), &config)
            } else {
                error!(
                    "The root of the unpacked form must be a directory, but '{}' only unpacks into a single file.",
                    mount.display()
                );
                std::process::exit(ERROR_STATUS_FUSE);
            }
        }
        Format::Toml => {
            let value = TomlValue::from_reader(reader);
            if value.is_dir() {
                unpack(value, mount.clone(), &config)
            } else {
                error!(
                    "The root of the unpacked form must be a directory, but '{}' only unpacks into a single file.",
                    mount.display()
                );
                std::process::exit(ERROR_STATUS_FUSE);
            }
        }
        Format::Yaml => {
            let value = YamlValue::from_reader(reader);
            if value.is_dir() {
                unpack(value, mount.clone(), &config)
            } else {
                error!(
                    "The root of the unpacked form must be a directory, but '{}' only unpacks into a single file.",
                    mount.display()
                );
                std::process::exit(ERROR_STATUS_FUSE);
            }
        }
    }
}
