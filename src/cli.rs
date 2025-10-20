use clap::{value_parser, Arg, Command};
use clap_complete::Shell;

/// The possible formats.
pub const POSSIBLE_FORMATS: [&str; 3] = ["json", "toml", "yaml"];

/// The possible name munging policies.
pub const MUNGE_POLICIES: [&str; 2] = ["filter", "rename"];

pub fn ffs() -> Command {
    Command::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::new("SHELL")
                .help("Generate shell completions (and exit)")
                .long("completions")
                .value_name("SHELL")
                .value_parser(value_parser!(Shell))
        )
        .arg(
            Arg::new("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short('q')
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::new("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::new("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short('d')
        )
        .arg(
            Arg::new("EAGER")
                .help("Eagerly load data on startup (data is lazily loaded by default)")
                .long("eager")
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
            Arg::new("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::new("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
        )
        .arg(
            Arg::new("KEEPMACOSDOT")
                .help("Include ._* extended attribute/resource fork files on macOS")
                .long("keep-macos-xattr")
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
            Arg::new("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
        )
        .arg(
            Arg::new("READONLY")
                .help("Mounted filesystem will be readonly")
                .long("readonly")
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
        )
        .arg(
            Arg::new("INPLACE")
                .help("Writes the output back over the input file")
                .long("in-place")
                .short('i')
                .overrides_with("OUTPUT")
                .overrides_with("NOOUTPUT")
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

pub fn unpack() -> Command {
    Command::new("unpack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("unpack structured data into a directory")
        .arg(
            Arg::new("SHELL")
                .help("Generate shell completions (and exit)")
                .long("completions")
                .value_name("SHELL")
                .value_parser(value_parser!(Shell))
        )
        .arg(
            Arg::new("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short('q')
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::new("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::new("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short('d')
        )
        .arg(
            Arg::new("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::new("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
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
            Arg::new("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
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

pub fn pack() -> Command {
    Command::new("pack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("pack directory")
        .arg(
            Arg::new("SHELL")
                .help("Generate shell completions (and exit)")
                .long("completions")
                .value_name("SHELL")
                .value_parser(value_parser!(Shell))
        )
        .arg(
            Arg::new("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short('q')
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::new("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::new("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short('d')
        )
        .arg(
            Arg::new("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::new("NOFOLLOW_SYMLINKS")
                .help("Never follow symbolic links. This is the default behaviour. `pack` will ignore all symbolic links.")
                .short('P')
                .overrides_with("FOLLOW_SYMLINKS")
        )
        .arg(
            Arg::new("FOLLOW_SYMLINKS")
                .help("Follow all symlinks. For safety, you can also specify a --max-depth value.")
                .short('L')
                .overrides_with("NOFOLLOW_SYMLINKS")
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
        )
        .arg(
            Arg::new("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
        )
        .arg(
            Arg::new("KEEPMACOSDOT")
                .help("Include ._* extended attribute/resource fork files on macOS")
                .long("keep-macos-xattr")
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
        )
        .arg(
            Arg::new("INPUT")
                .help("Sets the input folder")
                .index(1),
        )
}
