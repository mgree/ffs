use tracing::{error, info, warn};

mod cli;
mod config;
mod format;
mod fs;

use config::{Config, ERROR_STATUS_CLI, ERROR_STATUS_FUSE};

use fuser::MountOption;

fn main() {
    let config = Config::from_args();
    let mut options = vec![
//        MountOption::AutoUnmount,
        MountOption::FSName(format!("{}", config.input)),
    ];
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
            std::process::exit(ERROR_STATUS_CLI);
        }
    };
    let cleanup_mount = config.cleanup_mount;
    let input_format = config.input_format;
    let fs = input_format.load(config);

    info!("mounting on {:?} with options {:?}", mount, options);
    let status = match fuser::mount2(fs, &mount, &options) {
        Ok(()) => {
            info!("unmounted");
            0
        }
        Err(e) => {
            error!("I/O error: {}", e);
            ERROR_STATUS_FUSE
        }
    };

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

    std::process::exit(status);
}
