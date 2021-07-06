use fuser::FileType;
use std::path::{Path, PathBuf};

use tracing::{debug, error, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::EnvFilter, fmt};

use super::format;
use super::format::Format;

use super::cli;

/// Configuration information
///
/// See `cli.rs` for information on the actual command-line options; see
/// `main.rs` for how those connect to this structure.
///
/// NB I know this arrangement sucks, but `clap`'s automatic stuff isn't
/// adequate to express what I want here. Command-line interfaces are hard. 😢
#[derive(Debug)]
pub struct Config {
    pub input_format: Format,
    pub output_format: Format,
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
    pub read_only: bool,
    pub input: Input,
    pub output: Output,
    pub pretty: bool,
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

#[derive(Debug)]
pub enum Output {
    Quiet,
    Stdout,
    File(PathBuf),
}

impl Config {
    /// Parses arguments from `std::env::Args`, via `cli::app().get_matches()`
    pub fn from_args() -> Self {
        let args = cli::app().get_matches();

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
                std::process::exit(1);
            };
            cli::app().gen_completions_to("ffs", shell, &mut std::io::stdout());
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
        config.add_newlines = !args.is_present("EXACT");
        config.pad_element_names = !args.is_present("UNPADDED");
        config.read_only = args.is_present("READONLY");
        config.allow_xattr = !args.is_present("NOXATTR");
        config.keep_macos_xattr_file = args.is_present("KEEPMACOSDOT");
        config.pretty = args.is_present("PRETTY");

        // perms
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
                    std::process::exit(1);
                }
                let output = PathBuf::from(target_file);
                if output.exists() {
                    error!("Output file {} already exists.", output.display());
                    std::process::exit(1);
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
                                std::process::exit(1);
                            }
                        }
                    }
                };
                let mount = match args.value_of("MOUNT") {
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
                        // If the output is to a file foo.EXT, then try to make a directory foo.
                        let mount_dir = output.with_extension("");
                        // If that file already exists, give up and tell the user about --mount.
                        if mount_dir.exists() {
                            error!("Inferred mountpoint '{mount}' for output file '{file}', but '{mount}' already exists. Use `--mount MOUNT` to specify a mountpoint.", 
                                    mount = mount_dir.display(), file = output.display());
                            std::process::exit(1);
                        }
                        // If the mountpoint can't be created, give up and tell the user about --mount.
                        if let Err(e) = std::fs::create_dir(&mount_dir) {
                            error!("Couldn't create mountpoint '{}': {}. Use `--mount MOUNT` to specify a mountpoint.",
                                 mount_dir.display(),
                                    e
                                );
                            std::process::exit(1);
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
                                std::process::exit(1);
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
                            Input::Empty => {
                                error!(
                                    "You must specify a mount point when reading an empty file."
                                );
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

    pub fn normalize_name(&self, s: String) -> String {
        // inspired by https://en.wikipedia.org/wiki/Filename
        s.replace(".", "dot")
            .replace("/", "slash")
            .replace("\\", "backslash")
            .replace("?", "question")
            .replace("*", "star")
            .replace(":", "colon")
            .replace("\"", "dquote")
            .replace("<", "lt")
            .replace(">", "gt")
            .replace(",", "comma")
            .replace(";", "semi")
            .replace("=", "equal")
            .replace(" ", "space")
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_format: Format::Json,
            output_format: Format::Json,
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
            read_only: false,
            input: Input::Stdin,
            output: Output::Stdout,
            pretty: false,
            mount: None,
            cleanup_mount: false,
        }
    }
}
