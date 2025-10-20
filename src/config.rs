use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// use path_absolutize::*;

use clap_complete::{generate, Shell};
use tracing::{debug, error, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::EnvFilter, fmt};

use fuser::FileType;

use super::format;
use super::format::Format;

use super::cli;

pub const ERROR_STATUS_FUSE: i32 = 1;
pub const ERROR_STATUS_CLI: i32 = 2;

/// Configuration information
///
/// See `cli.rs` for information on the actual command-line options; see
/// `Config::from_args` for how those connect to this structure.
///
/// NB I know this arrangement sucks, but `clap`'s automatic stuff isn't
/// adequate to express what I want here. Command-line interfaces are hard. 😢
#[derive(Debug)]
pub struct Config {
    pub input_format: Format,
    pub output_format: Format,
    pub eager: bool,
    pub uid: u32,
    pub gid: u32,
    pub filemode: u16,
    pub dirmode: u16,
    pub add_newlines: bool,
    pub pad_element_names: bool,
    pub try_decode_base64: bool,
    pub allow_xattr: bool,
    pub keep_macos_xattr_file: bool,
    pub symlink: Symlink,
    pub max_depth: Option<u32>,
    pub allow_symlink_escape: bool,
    pub munge: Munge,
    pub read_only: bool,
    pub input: Input,
    pub output: Output,
    pub pretty: bool,
    pub timing: bool,
    pub mount: Option<PathBuf>,
    pub cleanup_mount: bool,
}

#[derive(Debug)]
pub enum Input {
    Stdin,
    File(PathBuf),
    Empty,
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Input::Empty => write!(f, "<empty>"),
            Input::Stdin => write!(f, "<stdin>"),
            Input::File(file) => write!(f, "{}", file.display()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Output {
    Quiet,
    Stdout,
    File(PathBuf),
}

#[derive(Debug)]
pub enum Munge {
    Rename,
    Filter,
}

impl std::fmt::Display for Munge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Munge::Rename => write!(f, "rename"),
            Munge::Filter => write!(f, "filter"),
        }
    }
}

impl FromStr for Munge {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let s = s.trim().to_lowercase();

        if s == "rename" {
            Ok(Munge::Rename)
        } else if s == "filter" {
            Ok(Munge::Filter)
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub enum Symlink {
    NoFollow,
    Follow,
}

impl Config {
    /// Parses arguments from `std::env::Args`, via `cli::app().get_matches()`
    pub fn from_ffs_args() -> Self {
        let args = cli::ffs().get_matches();

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(generator) = args.get_one::<Shell>("SHELL").copied() {
            let mut cmd = cli::ffs();
            generate(
                generator,
                &mut cmd,
      "ffs",
                &mut std::io::stdout(),
            );
            std::process::exit(0);
        }

        // logging
        if !args.contains_id("QUIET") {
            let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_e| {
                if args.contains_id("DEBUG") {
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

        // simple flags
        config.timing = args.contains_id("TIMING");
        config.eager = args.contains_id("EAGER");
        config.add_newlines = !args.contains_id("EXACT");
        config.pad_element_names = !args.contains_id("UNPADDED");
        config.read_only = args.contains_id("READONLY");
        config.allow_xattr = !args.contains_id("NOXATTR");
        config.keep_macos_xattr_file = args.contains_id("KEEPMACOSDOT");
        config.pretty = args.contains_id("PRETTY");

        // munging policy
        config.munge = match args.get_one::<String>("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{s}', using 'rename'.");
                    Munge::Rename
                }
            },
        };

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
        if args.contains_id("FILEMODE") && !args.contains_id("DIRMODE") {
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
            Some(gid) =>  config.gid = gid,
            None => config.gid = unsafe { libc::getegid() },
        }

        // two modes: with `--new` flag (infer most stuff) or without (parse other args)
        //
        // TODO 2021-07-06 maybe this would all be better with subcommands. but all that is so _complex_ :(
        match args.get_one::<String>("NEW") {
            Some(target_file) => {
                // `--new` flag, so we'll infer most stuff

                if args.contains_id("INPUT") {
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
                    .ok_or(format::ParseFormatError::NoFormatProvided)
                    .and_then(|s| s.parse::<Format>())
                {
                    Ok(target_format) => target_format,
                    Err(e) => {
                        match e {
                            format::ParseFormatError::NoSuchFormat(s) => {
                                warn!(
                                    "Unrecognized format '{s}', inferring from {}.",
                                    output.display(),
                                )
                            }
                            format::ParseFormatError::NoFormatProvided => {
                                debug!("Inferring output format from input.")
                            }
                        };
                        match output
                            .extension()
                            .and_then(|s| s.to_str())
                            .ok_or(format::ParseFormatError::NoFormatProvided)
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
                            error!("Inferred mountpoint '{mount}' for output file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.",
                                    mount = mount_dir.display(), file = output.display());
                            std::process::exit(ERROR_STATUS_FUSE);
                        }
                        // If the mountpoint can't be created, give up and tell the user about --mount.
                        if let Err(e) = std::fs::create_dir(&mount_dir) {
                            error!("Couldn't create mountpoint '{}': {e}. Use `--mount MOUNT` to specify a mountpoint.",
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
                } else if args.contains_id("INPLACE") {
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
                } else if args.contains_id("NOOUTPUT") || args.contains_id("QUIET") {
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
                                error!(
                                    "You must specify a mount point when reading an empty file."
                                );
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
                                    error!("Inferred mountpoint '{mount}' for input file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.",
                                    mount = mount_dir.display(), file = file.display());
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
                    .ok_or(format::ParseFormatError::NoFormatProvided)
                    .and_then(|s| s.parse::<Format>())
                {
                    Ok(source_format) => source_format,
                    Err(e) => {
                        match e {
                            format::ParseFormatError::NoSuchFormat(s) => {
                                warn!("Unrecognized format '{s}', inferring from input.")
                            }
                            format::ParseFormatError::NoFormatProvided => {
                                debug!("Inferring format from input.")
                            }
                        };
                        match &config.input {
                            Input::Stdin => Format::Json,
                            Input::Empty => Format::Json,
                            Input::File(input_source) => match input_source
                                .extension()
                                .and_then(|s| s.to_str())
                                .ok_or(format::ParseFormatError::NoFormatProvided)
                                .and_then(|s| s.parse::<Format>())
                            {
                                Ok(format) => format,
                                Err(e) => {
                                    match e {
                                        format::ParseFormatError::NoFormatProvided => {
                                            warn!("No extension detected, defaulting to JSON.")
                                        }
                                        format::ParseFormatError::NoSuchFormat(s) => {
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
                    .ok_or(format::ParseFormatError::NoFormatProvided)
                    .and_then(|s| s.parse::<Format>())
                {
                    Ok(target_format) => target_format,
                    Err(e) => {
                        match e {
                            format::ParseFormatError::NoSuchFormat(s) => {
                                warn!("Unrecognized format '{s}', inferring from input and output.")
                            }
                            format::ParseFormatError::NoFormatProvided => {
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

    pub fn from_unpack_args() -> Self {
        let args = cli::unpack().get_matches();

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(generator) = args.get_one::<Shell>("SHELL").copied() {
            let mut cmd = cli::unpack();
            generate(generator, &mut cmd, "unpack", &mut std::io::stdout());
            std::process::exit(0);
        }

        // logging
        if !args.contains_id("QUIET") {
            let filter_layer = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_e| {
                    if args.contains_id("DEBUG") {
                        EnvFilter::new("unpack=debug")
                    } else {
                        EnvFilter::new("unpack=warn")
                    }
                })
                .add_directive("ffs::config=warn".parse().unwrap());
            let fmt_layer = fmt::layer().with_writer(std::io::stderr);
            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }

        // simple flags
        config.timing = args.contains_id("TIMING");
        config.add_newlines = !args.contains_id("EXACT");
        config.pad_element_names = !args.contains_id("UNPADDED");
        config.allow_xattr = !args.contains_id("NOXATTR");

        // munging policy
        config.munge = match args.get_one::<String>("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{s}', using 'rename'.");
                    Munge::Rename
                }
            },
        };

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
                        error!("--new is not an option for `unpack`, so the input should never be Empty and this error should never be seen.");
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
                            error!("Inferred directory '{mount}' for input file '{file}', but '{mount}' already exists. Use `--into DIRECTORY` to specify a directory.",
                            mount = mount_dir.display(), file = file.display());
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
            .ok_or(format::ParseFormatError::NoFormatProvided)
            .and_then(|s| s.parse::<Format>())
        {
            Ok(source_format) => source_format,
            Err(e) => {
                match e {
                    format::ParseFormatError::NoSuchFormat(s) => {
                        warn!("Unrecognized format '{s}', inferring from input.")
                    }
                    format::ParseFormatError::NoFormatProvided => {
                        debug!("Inferring format from input.")
                    }
                };
                match &config.input {
                    Input::Stdin => Format::Json,
                    Input::Empty => Format::Json,
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

        config
    }

    pub fn from_pack_args() -> Self {
        let args = cli::pack().get_matches();

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(generator) = args.get_one::<Shell>("SHELL").copied() {
            let mut cmd = cli::pack();
            generate(generator, &mut cmd, "pack", &mut std::io::stdout());
            std::process::exit(0);
        }

        // logging
        if !args.contains_id("QUIET") {
            let filter_layer = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_e| {
                    if args.contains_id("DEBUG") {
                        EnvFilter::new("pack=debug")
                    } else {
                        EnvFilter::new("pack=warn")
                    }
                })
                .add_directive("ffs::config=warn".parse().unwrap());
            let fmt_layer = fmt::layer().with_writer(std::io::stderr);
            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }

        // simple flags
        config.timing = args.contains_id("TIMING");
        config.add_newlines = !args.contains_id("EXACT");
        config.read_only = args.contains_id("READONLY");
        config.allow_xattr = !args.contains_id("NOXATTR");
        config.allow_symlink_escape = args.contains_id("ALLOW_SYMLINK_ESCAPE");
        config.keep_macos_xattr_file = args.contains_id("KEEPMACOSDOT");
        config.pretty = args.contains_id("PRETTY");

        config.symlink = if args.contains_id("FOLLOW_SYMLINKS") {
            Symlink::Follow
        } else {
            Symlink::NoFollow
        };

        config.max_depth = args.get_one::<u32>("MAXDEPTH").copied();

        // munging policy
        config.munge = match args.get_one::<String>("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{s}', using 'rename'.");
                    Munge::Rename
                }
            },
        };

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
        } else if args.contains_id("NOOUTPUT") || args.contains_id("QUIET") {
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
            .ok_or(format::ParseFormatError::NoFormatProvided)
            .and_then(|s| s.parse::<Format>())
        {
            Ok(target_format) => target_format,
            Err(e) => {
                match e {
                    format::ParseFormatError::NoSuchFormat(s) => {
                        warn!("Unrecognized format '{s}', inferring from input and output.")
                    }
                    format::ParseFormatError::NoFormatProvided => {
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

    pub fn valid_name(&self, s: &str) -> bool {
        s != "." && s != ".." && !s.contains('\0') && !s.contains('/')
    }

    pub fn normalize_name(&self, s: String) -> String {
        if s == "." {
            "_.".into()
        } else if s == ".." {
            "_..".into()
        } else {
            s.replace('\0', "_NUL_").replace('/', "_SLASH_")
        }
    }

    #[cfg(target_os = "macos")]
    fn platform_ignored_file(&self, s: &str) -> bool {
        !self.keep_macos_xattr_file && s.starts_with("._")
    }

    #[cfg(target_os = "linux")]
    fn platform_ignored_file(&self, _s: &str) -> bool {
        false
    }

    /// Returns `true` for filenames that should not be serialized back.
    ///
    /// By default, this includes `.` and `..` (though neither of these occur in
    /// `FS` as `Inode`s). On macOS, filenames starting with `._` are ignored,
    /// as well---these are where macOS will store extended attributes on
    /// filesystems that don't support them.
    pub fn ignored_file(&self, s: &str) -> bool {
        s == "." || s == ".." || self.platform_ignored_file(s)
    }

    /// Determines the default mode of a file
    pub fn mode(&self, kind: FileType) -> u16 {
        if kind == FileType::Directory {
            self.dirmode
        } else {
            self.filemode
        }
    }

    /// Generate a reader for input
    ///
    /// A return of `None` means to start from an empty named directory
    pub fn input_reader(&self) -> Option<Box<dyn std::io::Read>> {
        match &self.input {
            Input::Stdin => Some(Box::new(std::io::stdin())),
            Input::File(file) => {
                let fmt = self.input_format;
                let file = std::fs::File::open(file).unwrap_or_else(|e| {
                    error!("Unable to open {} for {fmt} input: {e}", file.display());
                    std::process::exit(ERROR_STATUS_FUSE);
                });
                Some(Box::new(file))
            }
            Input::Empty => None,
        }
    }

    /// Generate a writer for output
    ///
    /// A return of `None` means no output should be provided
    pub fn output_writer(&self) -> Option<Box<dyn std::io::Write>> {
        match &self.output {
            Output::Stdout => {
                debug!("outputting on STDOUT");
                Some(Box::new(std::io::stdout()))
            }
            Output::File(path) => {
                debug!("output {}", path.display());
                Some(Box::new(File::create(path).unwrap()))
            }
            Output::Quiet => {
                debug!("no output path, skipping");
                None
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_format: Format::Json,
            output_format: Format::Json,
            eager: false,
            uid: 501,
            gid: 501,
            filemode: 0o644,
            dirmode: 0o755,
            add_newlines: true,
            pad_element_names: true,
            try_decode_base64: false,
            allow_xattr: true,
            keep_macos_xattr_file: false,
            symlink: Symlink::NoFollow,
            max_depth: None,
            allow_symlink_escape: false,
            munge: Munge::Rename,
            read_only: false,
            input: Input::Stdin,
            output: Output::Stdout,
            pretty: false,
            timing: false,
            mount: None,
            cleanup_mount: false,
        }
    }
}
