use clap::{App, Arg};

/// The possible formats.
pub const POSSIBLE_FORMATS: &[&str] = &["json", "toml", "yaml"];

/// The possible name munging policies.
pub const MUNGE_POLICIES: &[&str] = &["filter", "rename"];

pub fn ffs() -> App<'static, 'static> {
    App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::with_name("SHELL")
                .help("Generate shell completions (and exits)")
                .long("completions")
                .takes_value(true)
                .possible_values(&["bash", "fish", "zsh"])
        )
        .arg(
            Arg::with_name("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short("q")
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::with_name("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::with_name("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short("d")
        )
        .arg(
            Arg::with_name("EAGER")
                .help("Eagerly load data on startup (data is lazily loaded by default)")
                .long("eager")
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
                .help("Sets the default mode of files (parsed as octal)")
                .long("mode")
                .takes_value(true)
                .default_value("644")
        )
        .arg(
            Arg::with_name("DIRMODE")
                .help("Sets the default mode of directories (parsed as octal; if unspecified, directories will have FILEMODE with execute bits set when read bits are set)")
                .long("dirmode")
                .takes_value(true)
                .default_value("755")
        )
        .arg(
            Arg::with_name("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::with_name("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
        )
        .arg(
            Arg::with_name("KEEPMACOSDOT")
                .help("Include ._* extended attribute/resource fork files on macOS")
                .long("keep-macos-xattr")
        )
        .arg(
            Arg::with_name("MUNGE")
                .help("Set the name munging policy; applies to '.', '..', and files with NUL and '/' in them")
                .long("munge")
                .takes_value(true)
                .default_value("rename")
                .possible_values(MUNGE_POLICIES)
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
                .help("Disables output of filesystem (normally on stdout)")
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
                .possible_values(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("TARGET_FORMAT")
                .help("Specify the target format explicitly (by default, automatically inferred from filename extension)")
                .long("target")
                .short("t")
                .takes_value(true)
                .possible_values(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("PRETTY")
                .help("Pretty-print output (may increase size)")
                .long("pretty")
                .overrides_with("NOOUTPUT")
                .overrides_with("QUIET")
        )
        .arg(
            Arg::with_name("MOUNT")
                .help("Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin")
                .long("mount")
                .short("m")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("NEW")
                .help("Mounts an empty filesystem, inferring a mountpoint and output format")
                .long("new")
                .takes_value(true)
                .conflicts_with("INPLACE")
                .conflicts_with("SOURCE_FORMAT")
                .conflicts_with("OUTPUT")
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file ('-' means STDIN)")
                .default_value("-")
                .index(1),
        )
}

pub fn unpack() -> App<'static, 'static> {
    App::new("unpack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("unpack structured data into a directory")
        .arg(
            Arg::with_name("SHELL")
                .help("Generate shell completions (and exits)")
                .long("completions")
                .takes_value(true)
                .possible_values(&["bash", "fish", "zsh"])
        )
        .arg(
            Arg::with_name("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short("q")
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::with_name("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::with_name("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short("d")
        )
        .arg(
            Arg::with_name("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::with_name("NOXATTR")
                .help("Don't use extended attributes to track metadata (see `man xattr`)")
                .long("no-xattr")
        )
        .arg(
            Arg::with_name("MUNGE")
                .help("Set the name munging policy; applies to '.', '..', and files with NUL and '/' in them")
                .long("munge")
                .takes_value(true)
                .default_value("rename")
                .possible_values(MUNGE_POLICIES)
        )
        .arg(
            Arg::with_name("UNPADDED")
                .help("Don't pad the numeric names of list elements with zeroes; will not sort properly")
                .long("unpadded")
        )
        .arg(
            Arg::with_name("TYPE")
                .help("Specify the format type explicitly (by default, automatically inferred from filename extension)")
                .long("type")
                .short("t")
                .takes_value(true)
                .possible_values(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("INTO")
                .help("Sets the directory in which to unpack the file; will be inferred when using a file, but must be specified when running on stdin")
                .long("into")
                .short("i")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file ('-' means STDIN)")
                .default_value("-")
                .index(1),
        )
}

pub fn pack() -> App<'static, 'static> {
    App::new("pack")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("pack directory")
        .arg(
            Arg::with_name("SHELL")
                .help("Generate shell completions (and exits)")
                .long("completions")
                .takes_value(true)
                .possible_values(&["bash", "fish", "zsh"])
        )
        .arg(
            Arg::with_name("QUIET")
                .help("Quiet mode (turns off all errors and warnings, enables `--no-output`)")
                .long("quiet")
                .short("q")
                .overrides_with("DEBUG")
        )
        .arg(
            Arg::with_name("TIMING")
                .help("Emit timing information on stderr in an 'event,time' format; time is in nanoseconds")
                .long("time")
        )
        .arg(
            Arg::with_name("DEBUG")
                .help("Give debug output on stderr")
                .long("debug")
                .short("d")
        )
        .arg(
            Arg::with_name("EXACT")
                .help("Don't add newlines to the end of values that don't already have them (or strip them when loading)")
                .long("exact")
        )
        .arg(
            Arg::with_name("KEEPMACOSDOT")
                .help("Include ._* extended attribute/resource fork files on macOS")
                .long("keep-macos-xattr")
        )
        .arg(
            Arg::with_name("MUNGE")
                .help("Set the name munging policy; applies to '.', '..', and files with NUL and '/' in them")
                .long("munge")
                .takes_value(true)
                .default_value("rename")
                .possible_values(MUNGE_POLICIES)
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
                .help("Disables output of filesystem (normally on stdout)")
                .long("no-output")
                .overrides_with("OUTPUT")
        )
        .arg(
            Arg::with_name("TARGET_FORMAT")
                .help("Specify the target format explicitly (by default, automatically inferred from filename extension)")
                .long("target")
                .short("t")
                .takes_value(true)
                .possible_values(POSSIBLE_FORMATS)
        )
        .arg(
            Arg::with_name("PRETTY")
                .help("Pretty-print output (may increase size)")
                .long("pretty")
                .overrides_with("NOOUTPUT")
                .overrides_with("QUIET")
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input folder")
                .index(1),
        )
}
