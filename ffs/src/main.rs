use std::path::{Path, PathBuf};

use clap::parser::ValueSource;
use clap::{Arg, ArgAction, Command, value_parser};
use tracing::{debug, error, info, warn};

use nodelike::config::{
    Config, ERROR_STATUS_CLI, ERROR_STATUS_FUSE, Input, Output, POSSIBLE_FORMATS,
};
use nodelike::{Format, ParseFormatError};

use fuser::MountOption;

mod fs;
use fs::FS;

pub fn ffs_cli() -> Command {
    nodelike::config::cli_base("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::new("EAGER")
                .help("Eagerly load data on startup (data is lazily loaded by default)")
                .long("eager")
                .action(ArgAction::SetTrue)

        )
        .arg(
            Arg::new("UID")
                .help("Sets the user id of the generated filesystem (defaults to current effective user id)")
                .short('u')
                .long("uid")
                .value_name("UID")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("GID")
                .help("Sets the group id of the generated filesystem (defaults to current effective group id)")
                .short('g')
                .long("gid")
                .value_name("GID")
                .value_parser(value_parser!(u32)),

        )
        .arg(
            Arg::new("FILEMODE")
                .help("Sets the default mode of files (parsed as octal)")
                .long("mode")
                .value_name("FILEMODE")
                .default_value("644")
        )
        .arg(
            Arg::new("DIRMODE")
                .help("Sets the default mode of directories (parsed as octal; if unspecified, directories will have FILEMODE with execute bits set when read bits are set)")
                .long("dirmode")
                .value_name("DIRMODE")
                .default_value("755")
        )
        .arg(
            Arg::new("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("READONLY")
                .help("Mounted filesystem will be readonly")
                .long("readonly")
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
            Arg::new("INPLACE")
                .help("Writes the output back over the input file")
                .long("in-place")
                .short('i')
                .overrides_with("OUTPUT")
                .overrides_with("NOOUTPUT")
                .action(ArgAction::SetTrue)

        )
        .arg(
            Arg::new("SOURCE_FORMAT")
                .help("Specify the source format explicitly (by default, automatically inferred from filename extension)")
                .long("source")
                .short('s')
                .value_name("SOURCE_FORMAT")
                .value_parser(POSSIBLE_FORMATS)
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
            Arg::new("MOUNT")
                .help("Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin")
                .long("mount")
                .short('m')
                .value_name("MOUNT")
        )
        .arg(
            Arg::new("NEW")
                .help("Mounts an empty filesystem, inferring a mountpoint and output format")
                .long("new")
                .value_name("NEW")
                .conflicts_with("INPLACE")
                .conflicts_with("SOURCE_FORMAT")
                .conflicts_with("OUTPUT")
        )
        .arg(
            Arg::new("INPUT")
                .help("Sets the input file ('-' means STDIN)")
                .default_value("-")
                .index(1),
        )
}

/// Parses arguments from `std::env::Args`, via `cli::app().get_matches()`
pub fn config_from_ffs_args() -> Config {
    let (mut config, args) = Config::from_cli(ffs_cli);

    // simple flags
    config.eager = args.get_flag("EAGER");
    config.pad_element_names = !args.get_flag("UNPADDED");
    config.read_only = args.get_flag("READONLY");
    config.pretty = args.get_flag("PRETTY");

    // perms
    config.filemode = match u16::from_str_radix(args.get_one::<String>("FILEMODE").unwrap(), 8) {
        Ok(filemode) => filemode,
        Err(e) => {
            error!(
                "Couldn't parse `--mode {}`: {e}.",
                args.get_one::<String>("FILEMODE").unwrap()
            );
            std::process::exit(ERROR_STATUS_CLI)
        }
    };
    if args.value_source("FILEMODE") == Some(ValueSource::CommandLine)
        && args.value_source("DIRMODE") != Some(ValueSource::CommandLine)
    {
        // wherever a read bit is set, the dirmode should have an execute bit, too
        config.dirmode = config.filemode;
        if config.dirmode & 0o400 != 0 {
            config.dirmode |= 0o100;
        }
        if config.dirmode & 0o040 != 0 {
            config.dirmode |= 0o010;
        }
        if config.dirmode & 0o004 != 0 {
            config.dirmode |= 0o001;
        }
    } else {
        config.dirmode = match u16::from_str_radix(args.get_one::<String>("DIRMODE").unwrap(), 8) {
            Ok(filemode) => filemode,
            Err(e) => {
                error!(
                    "Couldn't parse `--dirmode {}`: {e}.",
                    args.get_one::<String>("DIRMODE").unwrap()
                );
                std::process::exit(ERROR_STATUS_CLI)
            }
        };
    }

    // uid and gid
    match args.get_one::<u32>("UID").copied() {
        Some(uid) => config.uid = uid,
        None => config.uid = unsafe { libc::geteuid() },
    }
    match args.get_one::<u32>("GID").copied() {
        Some(gid) => config.gid = gid,
        None => config.gid = unsafe { libc::getegid() },
    }

    // two modes: with `--new` flag (infer most stuff) or without (parse other args)
    //
    // TODO 2021-07-06 maybe this would all be better with subcommands. but all that is so _complex_ :(
    match args.get_one::<String>("NEW") {
        Some(target_file) => {
            // `--new` flag, so we'll infer most stuff

            if args.value_source("INPUT") == Some(ValueSource::CommandLine) {
                error!("It doesn't make sense to set `--new` with a specified input file.");
                std::process::exit(ERROR_STATUS_CLI);
            }
            let output = PathBuf::from(target_file);
            if output.exists() {
                error!("Output file {} already exists.", output.display());
                std::process::exit(ERROR_STATUS_FUSE);
            }
            let format = match args
                .get_one::<String>("TARGET_FORMAT")
                .ok_or(ParseFormatError::NoFormatProvided)
                .and_then(|s| s.parse::<Format>())
            {
                Ok(target_format) => target_format,
                Err(e) => {
                    match e {
                        ParseFormatError::NoSuchFormat(s) => {
                            warn!(
                                "Unrecognized format '{s}', inferring from {}.",
                                output.display(),
                            )
                        }
                        ParseFormatError::NoFormatProvided => {
                            debug!("Inferring output format from input.")
                        }
                    };
                    match output
                        .extension()
                        .and_then(|s| s.to_str())
                        .ok_or(ParseFormatError::NoFormatProvided)
                        .and_then(|s| s.parse::<Format>())
                    {
                        Ok(format) => format,
                        Err(_) => {
                            error!(
                                "Unrecognized format '{}'; use --target or a known extension to specify a format.",
                                output.display()
                            );
                            std::process::exit(ERROR_STATUS_CLI);
                        }
                    }
                }
            };
            let mount = match args.get_one::<String>("MOUNT") {
                Some(mount_point) => {
                    let mount_point = PathBuf::from(mount_point);
                    if !mount_point.exists() {
                        error!("Mount point {} does not exist.", mount_point.display());
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    config.cleanup_mount = false;
                    Some(mount_point)
                }
                None => {
                    // If the output is to a file foo.EXT, then try to make a directory foo.
                    let stem = output.file_stem().unwrap_or_else(|| {
                            error!("Couldn't infer the mountpoint from output '{}'. Use `--mount MOUNT` to specify a mountpoint.", output.display());
                            std::process::exit(ERROR_STATUS_FUSE);
                        });
                    let mount_dir = PathBuf::from(stem);
                    // If that file already exists, give up and tell the user about --mount.
                    if mount_dir.exists() {
                        error!(
                            "Inferred mountpoint '{mount}' for output file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.",
                            mount = mount_dir.display(),
                            file = output.display()
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    // If the mountpoint can't be created, give up and tell the user about --mount.
                    if let Err(e) = std::fs::create_dir(&mount_dir) {
                        error!(
                            "Couldn't create mountpoint '{}': {e}. Use `--mount MOUNT` to specify a mountpoint.",
                            mount_dir.display(),
                        );
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    // We did it!
                    config.cleanup_mount = true;
                    Some(mount_dir)
                }
            };
            config.input = Input::Empty;
            config.output = Output::File(output);
            config.input_format = format;
            config.output_format = format;
            config.mount = mount;
        }
        None => {
            // no `--new` flag... so parse everything

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

            // configure output
            config.output = if let Some(output) = args.get_one::<String>("OUTPUT") {
                Output::File(PathBuf::from(output))
            } else if args.get_flag("INPLACE") {
                match &config.input {
                    Input::Stdin => {
                        warn!(
                            "In-place output `-i` with STDIN input makes no sense; outputting on STDOUT."
                        );
                        Output::Stdout
                    }
                    Input::Empty => {
                        warn!(
                            "In-place output `-i` with empty input makes no sense; outputting on STDOUT."
                        );
                        Output::Stdout
                    }
                    Input::File(input_source) => Output::File(input_source.clone()),
                }
            } else if args.get_flag("NOOUTPUT") || args.get_flag("QUIET") {
                Output::Quiet
            } else {
                Output::Stdout
            };

            // infer and create mountpoint from filename as possible
            config.mount = match args.get_one::<String>("MOUNT") {
                Some(mount_point) => {
                    let mount_point = PathBuf::from(mount_point);
                    if !mount_point.exists() {
                        error!("Mount point {} does not exist.", mount_point.display());
                        std::process::exit(ERROR_STATUS_FUSE);
                    }
                    config.cleanup_mount = false;
                    Some(mount_point)
                }
                None => {
                    match &config.input {
                        Input::Stdin => {
                            error!("You must specify a mount point when reading from stdin.");
                            std::process::exit(ERROR_STATUS_CLI);
                        }
                        Input::Empty => {
                            error!("You must specify a mount point when reading an empty file.");
                            std::process::exit(ERROR_STATUS_CLI);
                        }
                        Input::File(file) => {
                            // If the input is from a file foo.EXT, then try to make a directory foo.
                            let stem = file.file_stem().unwrap_or_else(|| {
                                    error!("Couldn't infer the mountpoint from input '{}'. Use `--mount MOUNT` to specify a mountpoint.", file.display());
                                    std::process::exit(ERROR_STATUS_FUSE);
                                });
                            let mount_dir = PathBuf::from(stem);
                            debug!("inferred mount_dir {}", mount_dir.display());

                            // If that file already exists, give up and tell the user about --mount.
                            if mount_dir.exists() {
                                error!(
                                    "Inferred mountpoint '{mount}' for input file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.",
                                    mount = mount_dir.display(),
                                    file = file.display()
                                );
                                std::process::exit(ERROR_STATUS_FUSE);
                            }
                            // If the mountpoint can't be created, give up and tell the user about --mount.
                            if let Err(e) = std::fs::create_dir(&mount_dir) {
                                error!(
                                    "Couldn't create mountpoint '{}': {e}. Use `--mount MOUNT` to specify a mountpoint.",
                                    mount_dir.display()
                                );
                                std::process::exit(ERROR_STATUS_FUSE);
                            }
                            // We did it!
                            config.cleanup_mount = true;
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
                .get_one::<String>("SOURCE_FORMAT")
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
                            Err(e) => {
                                match e {
                                    ParseFormatError::NoFormatProvided => {
                                        warn!("No extension detected, defaulting to JSON.")
                                    }
                                    ParseFormatError::NoSuchFormat(s) => {
                                        warn!("Unrecognized extension {s}, defaulting to JSON.")
                                    }
                                };
                                Format::Json
                            }
                        },
                    }
                }
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

fn main() {
    let config = config_from_ffs_args();
    let mut options = vec![MountOption::FSName(format!("{}", config.input))];
    if config.read_only {
        options.push(MountOption::RO);
    }

    assert!(config.mount.is_some());
    let mount = match &config.mount {
        Some(mount) => mount.clone(),
        None => {
            error!(
                "No mount point specified; aborting. Use `--mount MOUNT` to specify a mountpoint."
            );
            std::process::exit(ERROR_STATUS_CLI);
        }
    };
    let cleanup_mount = config.cleanup_mount;
    let input_format = config.input_format;

    let status = match input_format {
        Format::Json => {
            let fs: FS<nodelike::json::Value> = FS::new(config);

            info!("mounting on {} with options {options:?}", mount.display());
            match fuser::mount2(fs, &mount, &options) {
                Ok(()) => {
                    info!("unmounted");
                    0
                }
                Err(e) => {
                    error!("I/O error: {e}");
                    ERROR_STATUS_FUSE
                }
            }
        }
        Format::Toml => {
            let fs: FS<nodelike::toml::Value> = FS::new(config);

            info!("mounting on {} with options {options:?}", mount.display());
            match fuser::mount2(fs, &mount, &options) {
                Ok(()) => {
                    info!("unmounted");
                    0
                }
                Err(e) => {
                    error!("I/O error: {e}");
                    ERROR_STATUS_FUSE
                }
            }
        }
        Format::Yaml => {
            let fs: FS<nodelike::yaml::Value> = FS::new(config);

            info!("mounting on {} with options {options:?}", mount.display());
            match fuser::mount2(fs, &mount, &options) {
                Ok(()) => {
                    info!("unmounted");
                    0
                }
                Err(e) => {
                    error!("I/O error: {e}");
                    ERROR_STATUS_FUSE
                }
            }
        }
    };

    if cleanup_mount {
        if mount.exists() {
            if let Err(e) = std::fs::remove_dir(&mount) {
                warn!("Unable to clean up mountpoint '{}': {e}", mount.display());
            }
        } else {
            warn!(
                "Mountpoint '{}' disappeared before ffs could cleanup.",
                mount.display()
            );
        }
    }

    std::process::exit(status);
}
