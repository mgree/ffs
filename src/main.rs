use std::path::Path;

use clap::{App, Arg};

use tracing::{error, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, fmt};

mod config;
mod fs;
mod parse;

use fuser::MountOption;
use serde_json::Value;

fn main() {
    let args = App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::with_name("AUTOUNMOUNT")
                .help("Automatically unmount the filesystem when the mounting process exits")
                .long("--autounmount"),
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

    let filter_layer = LevelFilter::DEBUG;
    let fmt_layer = fmt::layer();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let autounmount = args.is_present("AUTOUNMOUNT");

    // TODO 2021-06-08 infer and create mountpoint from filename as possible
    let mount_point = Path::new(args.value_of("MOUNT").expect("mount point"));
    if !mount_point.exists() {
        error!("Mount point {} does not exist.", mount_point.display());
        std::process::exit(1);
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

    let json: Value = parse::json(reader);
    let fs = fs::FS::from(json);

    let mut options = vec![MountOption::RO, MountOption::FSName(input_source.into())];
    if autounmount {
        options.push(MountOption::AutoUnmount);
    }
    info!("mounting on {:?} with options {:?}", mount_point, options);
    fuser::mount2(fs, mount_point, &options).unwrap();
    info!("unmounted");
}
