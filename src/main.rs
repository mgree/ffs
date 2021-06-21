use std::path::Path;
use std::path::PathBuf;

use clap::{App, Arg};

use tracing::{debug, error, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::EnvFilter, fmt};

mod config;
mod format;
mod fs;

use config::{Config, Output};
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
                .help("Sets the mountpoint")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file (defaults to '-', meaning STDIN)")
                .default_value("-")
                .index(2),
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

    // TODO 2021-06-08 infer and create mountpoint from filename as possible
    let mount_point = Path::new(args.value_of("MOUNT").expect("mount point"));
    if !mount_point.exists() {
        error!("Mount point {} does not exist.", mount_point.display());
        std::process::exit(1);
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

    let input_source = args.value_of("INPUT").expect("input source");
    config.output = if let Some(output) = args.value_of("OUTPUT") {
        Output::File(PathBuf::from(output))
    } else if args.is_present("INPLACE") {
        if input_source == "-" {
            warn!("In-place output `-i` with STDIN input makes no sense; outputting on STDOUT.");
            Output::Stdout
        } else {
            Output::File(PathBuf::from(input_source))
        }
    } else if args.is_present("NOOUTPUT") || args.is_present("QUIET") {
        Output::Quiet
    } else {
        Output::Stdout
    };

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

            match Path::new(input_source)
                .extension() // will fail for STDIN, no worries
                .map(|s| s.to_str().expect("utf8 filename").to_lowercase())
            {
                Some(s) => match s.parse::<Format>() {
                    Ok(format) => format,
                    Err(_) => {
                        warn!("Unrecognized format {}, defaulting to JSON.", s);
                        Format::Json
                    }
                },
                None => Format::Json,
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

    let mut options = vec![MountOption::FSName(input_source.into())];
    if args.is_present("AUTOUNMOUNT") {
        options.push(MountOption::AutoUnmount);
    }
    if config.read_only {
        options.push(MountOption::RO);
    }

    let input_format = config.input_format;
    let reader: Box<dyn std::io::Read> = if input_source == "-" {
        Box::new(std::io::stdin())
    } else {
        let file = std::fs::File::open(input_source).unwrap_or_else(|e| {
            error!("Unable to open {} for JSON input: {}", input_source, e);
            std::process::exit(1);
        });
        Box::new(file)
    };
    let fs = input_format.load(reader, config);

    info!("mounting on {:?} with options {:?}", mount_point, options);
    fuser::mount2(fs, mount_point, &options).unwrap();
    info!("unmounted");
}
