use std::path::{Path, PathBuf};

use clap::{App, Arg};

use tracing::{debug, error, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::EnvFilter, fmt};

mod config;
mod format;
mod fs;

use config::{Config, Input, Output};
use format::Format;

use fuser::MountOption;

fn main() {
    let args = App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::with_name("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short("q")
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::with_name("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short("d")
        )
        .arg(
            Arg::with_name("AUTOUNMOUNT")
                .help("Automatically unmount the filesystem when the mounting process exits")
                .long("autounmount"),
        )
        .arg(
            Arg::with_name("UID")
                .help("Sets the user id of the generated filesystem (defaults to current effective user id)")
                .short("u")
                .long("uid")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("GID")
                .help("Sets the group id of the generated filesystem (defaults to current effective group id)")
                .short("g")
                .long("gid")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILEMODE")
                .help("Sets the default mode of files (parsed as octal; defaults to 644; if unspecified, directories will have this mode with execute bits set when read bits are set)")
                .long("mode")
                .takes_value(true)
                .default_value("644")
        )
        .arg(
            Arg::with_name("DIRMODE")
                .help("Sets the default mode of directories (parsed as octal; defaults to 755; )")
                .long("dirmode")
                .takes_value(true)
                .default_value("755")
        )
        .arg(
            Arg::with_name("NEWLINE")
                .help("Add a newline to the end of values that don't already have them")
                .long("newline")
        )
        .arg(
            Arg::with_name("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
        )
        .arg(
            Arg::with_name("READONLY")
                .help("Mounted filesystem will be readonly")
                .long("readonly")
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file for saving changes (defaults to stdout)")
                .long("output")
                .short("o")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("NOOUTPUT")
                .help("Disables output")
                .long("no-output")
                .overrides_with("OUTPUT")
        )
        .arg(
            Arg::with_name("INPLACE")
                .help("Writes the output back over the input file")
                .long("in-place")
                .short("i")
                .overrides_with("OUTPUT")            
                .overrides_with("NOOUTPUT")
        )
        .arg(
            Arg::with_name("SOURCE_FORMAT")
                .help("Specify the source format explicitly (by default, automatically inferred from filename extension)")
                .long("source")
                .short("s")
                .takes_value(true)
                .possible_values(format::POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("TARGET_FORMAT")
                .help("Specify the target format explicitly (by default, automatically inferred from filename extension)")
                .long("target")
                .short("t")
                .takes_value(true)
                .possible_values(format::POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("MOUNT")
                .help("Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin")
                .long("mount")
                .short("m")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file (defaults to '-', meaning STDIN)")
                .default_value("-")
                .index(1),
        )
        .get_matches();

    let mut config = Config::default();

    if !args.is_present("QUIET") {
        let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_e| {
            if args.is_present("DEBUG") {
                EnvFilter::new("ffs=debug")
            } else {
                EnvFilter::new("ffs=warn")
            }
        });
        let fmt_layer = fmt::layer().with_writer(std::io::stderr);
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }

    config.add_newlines = args.is_present("NEWLINE");
    config.pad_element_names = !args.is_present("UNPADDED");
    config.read_only = args.is_present("READONLY");
    config.filemode = match u16::from_str_radix(args.value_of("FILEMODE").unwrap(), 8) {
        Ok(filemode) => filemode,
        Err(e) => {
            error!(
                "Couldn't parse `--mode {}`: {}.",
                args.value_of("FILEMODE").unwrap(),
                e
            );
            std::process::exit(1)
        }
    };
    if args.occurrences_of("FILEMODE") > 0 && args.occurrences_of("DIRMODE") == 0 {
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
        config.dirmode = match u16::from_str_radix(args.value_of("DIRMODE").unwrap(), 8) {
            Ok(filemode) => filemode,
            Err(e) => {
                error!(
                    "Couldn't parse `--dirmode {}`: {}.",
                    args.value_of("DIRMODE").unwrap(),
                    e
                );
                std::process::exit(1)
            }
        };
    }

    match args.value_of("UID") {
        Some(uid_string) => match uid_string.parse() {
            Ok(uid) => config.uid = uid,
            Err(e) => {
                let euid = unsafe { libc::geteuid() };
                warn!(
                    "Couldn't parse '{}' as a uid ({}), defaulting to effective uid ({})",
                    uid_string, e, euid
                );
                config.uid = euid;
            }
        },
        None => config.uid = unsafe { libc::geteuid() },
    }
    match args.value_of("GID") {
        Some(gid_string) => match gid_string.parse() {
            Ok(gid) => config.gid = gid,
            Err(e) => {
                let egid = unsafe { libc::getegid() };
                warn!(
                    "Couldn't parse '{}' as a gid ({}), defaulting to effective gid ({})",
                    gid_string, e, egid
                );
                config.gid = egid;
            }
        },
        None => config.gid = unsafe { libc::getegid() },
    }

    config.input = match args.value_of("INPUT") {
        Some(input_source) => {
            if input_source == "-" {
                Input::Stdin
            } else {
                let input_source = PathBuf::from(input_source);
                if !input_source.exists() {
                    error!("Input file {} does not exist.", input_source.display());
                    std::process::exit(1);
                }
                Input::File(input_source)
            }
        }
        None => Input::Stdin,
    };
    config.output = if let Some(output) = args.value_of("OUTPUT") {
        Output::File(PathBuf::from(output))
    } else if args.is_present("INPLACE") {
        match &config.input {
            Input::Stdin => {
                warn!(
                    "In-place output `-i` with STDIN input makes no sense; outputting on STDOUT."
                );
                Output::Stdout
            }
            Input::File(input_source) => Output::File(input_source.clone()),
        }
    } else if args.is_present("NOOUTPUT") || args.is_present("QUIET") {
        Output::Quiet
    } else {
        Output::Stdout
    };

    // infer and create mountpoint from filename as possible
    config.mount = match args.value_of("MOUNT") {
        Some(mount_point) => {
            let mount_point = PathBuf::from(mount_point);
            if !mount_point.exists() {
                error!("Mount point {} does not exist.", mount_point.display());
                std::process::exit(1);
            }

            config.cleanup_mount = false;
            Some(mount_point)
        }
        None => {
            match &config.input {
                Input::Stdin => {
                    error!("You must specify a mount point when reading from stdin.");
                    std::process::exit(1);
                }
                Input::File(file) => {
                    // If the input is from a file foo.EXT, then try to make a directory foo.
                    let mount_dir = file.with_extension("");

                    // If that file already exists, give up and tell the user about --mount.
                    if mount_dir.exists() {
                        error!("Inferred mountpoint '{mount}' for input file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.", 
                            mount = mount_dir.display(), file = file.display());
                        std::process::exit(1);
                    }

                    // If the mountpoint can't be created, give up and tell the user about --mount.
                    if let Err(e) = std::fs::create_dir(&mount_dir) {
                        error!(
                            "Couldn't create mountpoint '{}': {}. Use `--mount MOUNT` to specify a mountpoint.",
                            mount_dir.display(),
                            e
                        );
                        std::process::exit(1);
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
        .value_of("SOURCE_FORMAT")
        .ok_or(format::ParseFormatError::NoFormatProvided)
        .and_then(|s| s.parse::<Format>())
    {
        Ok(source_format) => source_format,
        Err(e) => {
            match e {
                format::ParseFormatError::NoSuchFormat(s) => {
                    warn!("Unrecognized format '{}', inferring from input.", s)
                }
                format::ParseFormatError::NoFormatProvided => {
                    debug!("Inferring format from input.")
                }
            };

            match &config.input {
                Input::Stdin => Format::Json,
                Input::File(input_source) => match input_source
                    .extension()
                    .and_then(|s| s.to_str())
                    .ok_or(format::ParseFormatError::NoFormatProvided)
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

    // try to autodetect the input format.
    //
    // first see if it's specified and parses okay.
    //
    // then see if we can pull it out of the extension (if specified)
    //
    // then give up and use the input format
    config.output_format = match args
        .value_of("TARGET_FORMAT")
        .ok_or(format::ParseFormatError::NoFormatProvided)
        .and_then(|s| s.parse::<Format>())
    {
        Ok(target_format) => target_format,
        Err(e) => {
            match e {
                format::ParseFormatError::NoSuchFormat(s) => {
                    warn!(
                        "Unrecognized format '{}', inferring from input and output.",
                        s
                    )
                }
                format::ParseFormatError::NoFormatProvided => {
                    debug!("Inferring output format from input.")
                }
            };

            match args
                .value_of("OUTPUT")
                .and_then(|s| Path::new(s).extension())
                .and_then(|s| s.to_str())
            {
                Some(s) => match s.parse::<Format>() {
                    Ok(format) => format,
                    Err(_) => {
                        warn!(
                            "Unrecognized format {}, defaulting to input format '{}'.",
                            s, config.input_format
                        );
                        config.input_format
                    }
                },
                None => config.input_format,
            }
        }
    };

    ////////////////////////////////////////////////////////////////////////////
    // DONE PARSING
    ////////////////////////////////////////////////////////////////////////////

    let mut options = vec![MountOption::FSName(format!("{}", config.input))];
    if args.is_present("AUTOUNMOUNT") {
        options.push(MountOption::AutoUnmount);
    }
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
            std::process::exit(1);
        }
    };
    let cleanup_mount = config.cleanup_mount;
    let input_format = config.input_format;
    let reader: Box<dyn std::io::Read> = match &config.input {
        Input::Stdin => Box::new(std::io::stdin()),
        Input::File(file) => {
            let fmt = config.input_format;
            let file = std::fs::File::open(&file).unwrap_or_else(|e| {
                error!("Unable to open {} for {} input: {}", file.display(), fmt, e);
                std::process::exit(1);
            });
            Box::new(file)
        }
    };
    let fs = input_format.load(reader, config);

    info!("mounting on {:?} with options {:?}", mount, options);
    fuser::mount2(fs, &mount, &options).unwrap();
    info!("unmounted");

    if cleanup_mount {
        if mount.exists() {
            if let Err(e) = std::fs::remove_dir(&mount) {
                warn!("Unable to clean up mountpoint '{}': {}", mount.display(), e);
            }
        } else {
            warn!(
                "Mountpoint '{}' disappeared before ffs could cleanup.",
                mount.display()
            );
        }
    }
}
