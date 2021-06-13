use std::path::Path;

use clap::{App, Arg};

use tracing::{error, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt};

mod config;
mod fs;
mod json;

use config::Config;

use fuser::MountOption;

fn main() {
    let args = App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
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
            Arg::with_name("NEWLINE")
            .help("Add a newline to the end of values that don't already have them")
            .long("newline")
            .default_value("false")
        )
        .arg(
            Arg::with_name("PADDED")
            .help("Pad the numeric names of list elements with zeroes, for proper sorting (e.g., 00, 01, ..., 09, 10)")
            .long("padded")
            .default_value("true")
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

    let filter_layer = LevelFilter::DEBUG;
    let fmt_layer = fmt::layer();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    config.add_newlines = match args.value_of("NEWLINE") {
        Some("true") | None => true,
        Some("false") => false,
        Some(s) => panic!("Got `--newline {}`; please give either `true` or `false`.", s),
    };
    config.pad_element_names = match args.value_of("PADDED") {
        Some("true") | None => true,
        Some("false") => false,
        Some(s) => panic!("Got `--padded {}`; please give either `true` or `false`.", s),
    };
    let autounmount = args.is_present("AUTOUNMOUNT");

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

    let reader: Box<dyn std::io::BufRead> = if input_source == "-" {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    } else {
        let file = std::fs::File::open(input_source).unwrap_or_else(|e| {
            error!("Unable to open {} for JSON input: {}", input_source, e);
            std::process::exit(1);
        });
        Box::new(std::io::BufReader::new(file))
    };

    let v = json::parse(reader);
    let fs = json::fs(config, v);

    let mut options = vec![MountOption::RO, MountOption::FSName(input_source.into())];
    if autounmount {
        options.push(MountOption::AutoUnmount);
    }
    info!("mounting on {:?} with options {:?}", mount_point, options);
    fuser::mount2(fs, mount_point, &options).unwrap();
    info!("unmounted");
}
