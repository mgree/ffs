use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
    pub base64: base64::Config,
    pub try_decode_base64: bool,
    pub allow_xattr: bool,
    pub keep_macos_xattr_file: bool,
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

impl Config {
    /// Parses arguments from `std::env::Args`, via `cli::app().get_matches()`
    pub fn from_ffs_args() -> Self {
        let args = cli::ffs().get_matches_safe().unwrap_or_else(|e| {
            eprintln!("{}", e.message);
            std::process::exit(ERROR_STATUS_CLI)
        });

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(shell) = args.value_of("SHELL") {
            let shell = if shell == "bash" {
                clap::Shell::Bash
            } else if shell == "fish" {
                clap::Shell::Fish
            } else if shell == "zsh" {
                clap::Shell::Zsh
            } else {
                eprintln!("Can't generate completions for '{}'.", shell);
                std::process::exit(ERROR_STATUS_CLI);
            };
            cli::ffs().gen_completions_to("ffs", shell, &mut std::io::stdout());
            std::process::exit(0);
        }

        // logging
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

        // simple flags
        config.timing = args.is_present("TIMING");
        config.eager = args.is_present("EAGER");
        config.add_newlines = !args.is_present("EXACT");
        config.pad_element_names = !args.is_present("UNPADDED");
        config.read_only = args.is_present("READONLY");
        config.allow_xattr = !args.is_present("NOXATTR");
        config.keep_macos_xattr_file = args.is_present("KEEPMACOSDOT");
        config.pretty = args.is_present("PRETTY");

        // munging policy
        config.munge = match args.value_of("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{}', using 'rename'.", s);
                    Munge::Rename
                }
            },
        };

        // perms
        config.filemode = match u16::from_str_radix(args.value_of("FILEMODE").unwrap(), 8) {
            Ok(filemode) => filemode,
            Err(e) => {
                error!(
                    "Couldn't parse `--mode {}`: {}.",
                    args.value_of("FILEMODE").unwrap(),
                    e
                );
                std::process::exit(ERROR_STATUS_CLI)
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
                    std::process::exit(ERROR_STATUS_CLI)
                }
            };
        }

        // uid and gid
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

        // two modes: with `--new` flag (infer most stuff) or without (parse other args)
        //
        // TODO 2021-07-06 maybe this would all be better with subcommands. but all that is so _complex_ :(
        match args.value_of("NEW") {
            Some(target_file) => {
                // `--new` flag, so we'll infer most stuff

                if args.occurrences_of("INPUT") != 0 {
                    error!("It doesn't make sense to set `--new` with a specified input file.");
                    std::process::exit(ERROR_STATUS_CLI);
                }
                let output = PathBuf::from(target_file);
                if output.exists() {
                    error!("Output file {} already exists.", output.display());
                    std::process::exit(ERROR_STATUS_FUSE);
                }
                let format = match args
                    .value_of("TARGET_FORMAT")
                    .ok_or(format::ParseFormatError::NoFormatProvided)
                    .and_then(|s| s.parse::<Format>())
                {
                    Ok(target_format) => target_format,
                    Err(e) => {
                        match e {
                            format::ParseFormatError::NoSuchFormat(s) => {
                                warn!(
                                    "Unrecognized format '{}', inferring from {}.",
                                    s,
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
                let mount = match args.value_of("MOUNT") {
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
                            error!("Couldn't create mountpoint '{}': {}. Use `--mount MOUNT` to specify a mountpoint.",
                                 mount_dir.display(),
                                    e
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
                config.input = match args.value_of("INPUT") {
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
                        Input::Empty => {
                            warn!(
                                "In-place output `-i` with empty input makes no sense; outputting on STDOUT."
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
                                        "Couldn't create mountpoint '{}': {}. Use `--mount MOUNT` to specify a mountpoint.",
                                        mount_dir.display(),
                                        e
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

                // try to autodetect the output format.
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
        let args = cli::unpack().get_matches_safe().unwrap_or_else(|e| {
            eprintln!("{}", e.message);
            std::process::exit(ERROR_STATUS_CLI)
        });

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(shell) = args.value_of("SHELL") {
            let shell = if shell == "bash" {
                clap::Shell::Bash
            } else if shell == "fish" {
                clap::Shell::Fish
            } else if shell == "zsh" {
                clap::Shell::Zsh
            } else {
                eprintln!("Can't generate completions for '{}'.", shell);
                std::process::exit(ERROR_STATUS_CLI);
            };
            cli::unpack().gen_completions_to("ffs", shell, &mut std::io::stdout());
            std::process::exit(0);
        }

        // logging
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

        // simple flags
        config.timing = args.is_present("TIMING");
        config.add_newlines = !args.is_present("EXACT");
        config.pad_element_names = !args.is_present("UNPADDED");
        // config.read_only = args.is_present("READONLY");  TODO (nad) 2023-04-04 maybe handle readonly
        config.allow_xattr = !args.is_present("NOXATTR");

        // munging policy
        config.munge = match args.value_of("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{}', using 'rename'.", s);
                    Munge::Rename
                }
            },
        };

        // configure input
        config.input = match args.value_of("INPUT") {
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
        config.mount = match args.value_of("INTO") {
            Some(mount_point) => {
                match std::fs::create_dir(&mount_point) {
                    Ok(_) => Some(PathBuf::from(mount_point)),
                    Err(_) => {
                        // if dir is empty then we can use it
                        let mount = PathBuf::from(mount_point);
                        if mount.read_dir().unwrap().next().is_none() {
                            // dir exists and is empty
                            Some(PathBuf::from(mount_point))
                        } else {
                            // dir exists but is not empty
                            error!("Directory `{}` already exists and is not empty.", mount_point);
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
                            "You must specify a mount point when reading an empty file."
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
                            error!("Inferred directory '{mount}' for input file '{file}', but '{mount}' already exists. Use `--into DIRECTORY` to specify a directory.",
                            mount = mount_dir.display(), file = file.display());
                            std::process::exit(ERROR_STATUS_FUSE);
                        }
                        // If the mountpoint can't be created, give up and tell the user about --mount.
                        if let Err(e) = std::fs::create_dir(&mount_dir) {
                            error!(
                                "Couldn't create directory '{}': {}. Use `--into DIRECTORY` to specify a directory.",
                                mount_dir.display(),
                                e
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
            .value_of("TYPE")
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
        let args = cli::pack().get_matches_safe().unwrap_or_else(|e| {
            eprintln!("{}", e.message);
            std::process::exit(ERROR_STATUS_CLI)
        });

        let mut config = Config::default();
        // generate completions?
        //
        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(shell) = args.value_of("SHELL") {
            let shell = if shell == "bash" {
                clap::Shell::Bash
            } else if shell == "fish" {
                clap::Shell::Fish
            } else if shell == "zsh" {
                clap::Shell::Zsh
            } else {
                eprintln!("Can't generate completions for '{}'.", shell);
                std::process::exit(ERROR_STATUS_CLI);
            };
            cli::pack().gen_completions_to("ffs", shell, &mut std::io::stdout());
            std::process::exit(0);
        }

        // logging
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

        // simple flags
        config.timing = args.is_present("TIMING");
        config.read_only = args.is_present("READONLY");
        config.allow_xattr = !args.is_present("NOXATTR");
        config.keep_macos_xattr_file = args.is_present("KEEPMACOSDOT");
        config.pretty = args.is_present("PRETTY");

        // munging policy
        config.munge = match args.value_of("MUNGE") {
            None => Munge::Filter,
            Some(s) => match str::parse(s) {
                Ok(munge) => munge,
                Err(_) => {
                    warn!("Invalid `--munge` mode '{}', using 'rename'.", s);
                    Munge::Rename
                }
            },
        };

        // configure input
        config.input = match args.value_of("INPUT") {
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
            Input::File(file) => Some(file.clone()),
            _ => {
                error!("Input must be a directory.");
                std::process::exit(ERROR_STATUS_CLI);
            }
        };

        // configure output
        config.output = if let Some(output) = args.value_of("OUTPUT") {
            Output::File(PathBuf::from(output))
        } else if args.is_present("NOOUTPUT") || args.is_present("QUIET") {
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
            s.replace("\0", "_NUL_").replace("/", "_SLASH_")
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
                let file = std::fs::File::open(&file).unwrap_or_else(|e| {
                    error!("Unable to open {} for {} input: {}", file.display(), fmt, e);
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
            base64: base64::STANDARD,
            try_decode_base64: false,
            allow_xattr: true,
            keep_macos_xattr_file: false,
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
