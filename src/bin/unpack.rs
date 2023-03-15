use std::collections::VecDeque;
use std::fs;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;

use ffs::config::Config;
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
    queue.push_back((root, root_path, None));

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
                // TODO(mmg) 2023-03-06 set `user.type` metadata using setxattr
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
                // TODO(mmg) 2023-03-06 set `user.type` metadata using setxattr
                xattr::set(&path, "user.type", format!("{}", Typ::Bytes).as_bytes())?;
            }
            format::Node::List(vs) => {
                // make directory
                fs::create_dir(&path)?;
                // TODO(mmg) 2023-03-06 set directory metadata to list using setxattr
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
                // make directory
                fs::create_dir(&path)?;
                // TODO(mmg) 2023-03-06 set directory metadata to map using setxattr
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

    let path = match &config.input {
        Input::File(path) => path,
        _ => {
            panic!("for testing, input must be a file");
        }
    };
    // println!("path: {:?}", path);
    let file = fs::File::open(&path)?;
    // println!("file: {:?}", file);
    let reader = Box::new(BufReader::new(file));
    // println!("reader: {:?}", reader);

    // TODO add subdirectory check not just root directory check

    match &config.input_format {
        Format::Json => unpack(JsonValue::from_reader(reader), mount, &config),
        Format::Toml => unpack(TomlValue::from_reader(reader), mount, &config),
        Format::Yaml => unpack(YamlValue::from_reader(reader), mount, &config),
    }
}
