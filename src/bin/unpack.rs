use tracing::{error, info, warn};

use std::collections::VecDeque;
use std::fs;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;

use ffs::config::Config;
use ffs::config::{ERROR_STATUS_CLI, ERROR_STATUS_FUSE};
use ffs::config::Input;
use ffs::format;
use format::{Format, Nodelike, Typ};
use format::json::Value as JsonValue;
use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;

use ::xattr;

#[allow(dead_code)]
fn unpack<V>(root: V, root_path: PathBuf, config: &Config) -> std::io::Result<()>
where
    V: Nodelike + std::fmt::Display + Default,
{
    let mut queue: VecDeque<(V, PathBuf, Option<String>)> = VecDeque::new();
    queue.push_back((root, root_path.clone(), None));

    while !queue.is_empty() {
        let (v, path, original_name) = queue.pop_front().unwrap();

        match v.node(config) {
            format::Node::String(t, s) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(&path)?;

                // write `s` into that file
                write!(f, "{}", s)?;

                // set metadata according to `t`
                xattr::set(&path, "user.type", format!("{}", t).as_bytes())?;
            }
            format::Node::Bytes(b) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(&path)?;

                // write `b` into that file
                f.write_all(b.as_slice())?;

                // set metadata to bytes
                xattr::set(&path, "user.type", format!("{}", Typ::Bytes).as_bytes())?;
            }
            format::Node::List(vs) => {
                // if not root path, make directory
                if path != root_path.clone() {
                    fs::create_dir(&path)?;
                }
                xattr::set(&path, "user.type", "list".as_bytes())?;

                // enqueue children with appropriate names
                let num_elts = vs.len() as f64;
                let width = num_elts.log10().ceil() as usize;

                for (i, child) in vs.into_iter().enumerate() {
                    // TODO(mmg) 2021-06-08 ability to add prefixes
                    let name = if config.pad_element_names {
                        format!("{:0width$}", i, width = width)
                    } else {
                        format!("{}", i)
                    };
                    let child_path = path.join(name);

                    queue.push_back((child, child_path, None));
                }
            }
            format::Node::Map(fvs) => {
                // if not root path, make directory
                if path != root_path.clone() {
                    fs::create_dir(&path)?;
                }
                xattr::set(&path, "user.type", "map".as_bytes())?;

                // enqueue children with appropriate names
                let mut child_names = std::collections::HashSet::new();
                for (field, child) in fvs.into_iter() {
                    let original = field.clone();

                    // munge name to be valid and unique
                    let name = if !config.valid_name(&original) {
                        match config.munge {
                            ffs::config::Munge::Rename => {
                                let mut nfield = config.normalize_name(field);

                                while child_names.contains(&nfield) {
                                    nfield.push('_');
                                }

                                nfield
                            }
                            ffs::config::Munge::Filter => {
                                // TODO(mmg) 2023-03-06 support logging
                                //warn!("skipping '{}'", field);
                                continue;
                            }
                        }
                    } else {
                        field
                    };
                    child_names.insert(name.clone());

                    let child_path = path.join(name);
                    queue.push_back((child, child_path, Some(original)));
                }
            }
        }

        if let Some(_original_name) = original_name {
            // TODO(mmg) 2023-03-6 set `user.original_name` using setxattr
            xattr::set(&path, "user.original_name", _original_name.as_bytes())?;
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let config = Config::from_unpack_args();
    // println!("{:?}", config);

    assert!(config.mount.is_some());
    let mount = match &config.mount {
        Some(mount) => mount.clone(),
        None => {
            panic!("Directory not specified");
        }
    };
    // println!("mount: {:?}", mount);

    let reader = match config.input_reader() {
        Some(reader) => reader,
        None => {
            error!("Input not specified");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };
    // println!("reader: {:?}", reader);

    // TODO (nad) add subdirectory check not just root directory check
    // TODO (nad) 2023-03-16 fix the amount of clones!!!
    let result = match &config.input_format {
        Format::Json => {
            // TODO (nad) 2023-04-11 if we want a nice format mismatch error message, we need to
            // replace the .expect("") call in format.rs with a new match statement.
            unpack(JsonValue::from_reader(reader), mount.clone(), &config)
        }
        Format::Toml => {
            unpack(TomlValue::from_reader(reader), mount.clone(), &config)
        }
        Format::Yaml => {
            unpack(YamlValue::from_reader(reader), mount.clone(), &config)
        }
    };

    // if readonly is supported,
    // have to set read_only in the root dir after unpacking because you can't create files or dirs
    // inside a read-only directory
    // if config.read_only {
    //     let file = fs::File::open(mount)?;
    //     let mut perms = file.metadata()?.permissions();
    //     perms.set_readonly(true);
    //     file.set_permissions(perms)?;
    // }

    result
}
