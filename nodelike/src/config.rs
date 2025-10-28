use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

use crate::Format;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use clap_complete::{Shell, generate};
use tracing::{debug, error, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub const ERROR_STATUS_FUSE: i32 = 1;
pub const ERROR_STATUS_CLI: i32 = 2;

/// The possible formats.
pub const POSSIBLE_FORMATS: [&str; 3] = ["json", "toml", "yaml"];

/// The possible name munging policies.
pub const MUNGE_POLICIES: [&str; 2] = ["filter", "rename"];

/// Common clap configuration
pub fn cli_base(name: impl Into<clap::builder::Str>) -> clap::Command {
    Command::new(name)
        .arg(
            Arg::new("SHELL")
                .help("Generate shell completions (and exit)")
                .long("completions")
                .value_name("SHELL")
                .value_parser(value_parser!(Shell)),
        )
        .arg(
            Arg::new("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short('q')
                .overrides_with("DEBUG")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short('d')
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
                .action(ArgAction::SetTrue)

        )
        .arg(
            Arg::new("MUNGE")
                .help("Set the name munging policy; applies to '.', '..', and files with NUL and '/' in them")
                .long("munge")
                .value_name("MUNGE")
                .default_value("rename")
                .value_parser(MUNGE_POLICIES)
        )
        .arg(
            Arg::new("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
                .action(ArgAction::SetTrue)

        )
}

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
    pub fn from_cli<F: Fn() -> Command>(gen_cli: F) -> (Self, ArgMatches) {
        let args = gen_cli().get_matches_mut();
        let mut config = Config::default();

        // TODO 2021-07-06 good candidate for a subcommand
        if let Some(generator) = args.get_one::<Shell>("SHELL").copied() {
            let mut cmd = gen_cli();
            let name = cmd.get_name().to_string();
            generate(generator, &mut cmd, name, &mut std::io::stdout());
            std::process::exit(0);
        }

        if !args.get_flag("QUIET") {
            let filter_layer = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_e| {
                    if args.get_flag("DEBUG") {
                        EnvFilter::new("pack=debug")
                    } else {
                        EnvFilter::new("pack=warn")
                    }
                })
                .add_directive("ffs::config=warn".parse().unwrap());
            let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }

        config.timing = args.get_flag("TIMING");
        config.add_newlines = !args.get_flag("EXACT");
        config.allow_xattr = !args.get_flag("NOXATTR");

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

        (config, args)
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
