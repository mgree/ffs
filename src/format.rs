use std::collections::HashMap;
use std::fs::File;
use std::str::FromStr;

use tracing::{debug, info, instrument};

use fuser::FileType;

use super::config::{Config, Output};
use super::fs::{DirEntry, DirType, Entry, Inode, FS};

#[derive(Copy, Clone, Debug)]
pub enum Format {
    Json,
}

impl Format {
    /// Generates a filesystem `fs`, reading from `reader` according to a
    /// particular `Config`.
    /// 
    /// NB there is no check that `self == fs.config.input_format`!
    #[instrument(level = "info", skip(reader, config))]
    pub fn load(&self, reader: Box<dyn std::io::BufRead>, config: Config) -> FS {
        let mut inodes: Vec<Option<Inode>> = Vec::new();

        match self {
            Format::Json => {
                info!("reading json value");
                let v = serde_json::from_reader(reader).expect("JSON");
                info!("building inodes");
                json::fs_from_value(v, &config, &mut inodes);
                info!("done");
            }
        };

        info!("done");
        FS::new(inodes, config)
    }

    /// Given a filesystem `fs`, it outputs a file in the appropriate format,
    /// following `fs.config`.
    /// 
    /// NB there is no check that `self == fs.config.output_format`!
    #[instrument(level = "info", skip(fs))]
    pub fn save(&self, fs: &FS) {
        let writer: Box<dyn std::io::Write> = match &fs.config.output {
            Output::Stdout => {
                debug!("outputting on STDOUT");
                Box::new(std::io::stdout())
            }
            Output::File(path) => {
                debug!("output {}", path.display());
                Box::new(File::create(path).unwrap())
            }
            Output::Quiet => {
                debug!("no output path, skipping");
                return;
            }
        };

        match self {
            Format::Json => {
                info!("generating json value");
                let v = json::value_from_fs(fs, fuser::FUSE_ROOT_ID);
                info!("writing");
                debug!("outputting {}", v);
                serde_json::to_writer(writer, &v).unwrap();
                info!("done")
            }
        }
    }
}

mod json {
    use super::*;
    use serde_json::Value;

    /// Given a JSON value `v`, initializes the vector `inodes` of (nullable)
    /// `Inodes` according to a given `config`.
    ///     
    /// The current implementation is eager: it preallocates enough inodes and then
    /// fills them in using a depth-first traversal.
    ///
    /// Invariant: the index in the vector is the inode number. Inode 0 is invalid,
    /// and is left empty.
    pub fn fs_from_value(v: Value, config: &Config, inodes: &mut Vec<Option<Inode>>) {
        // reserve space for everyone else
        // won't work with streaming or lazy generation, but avoids having to resize the vector midway through
        inodes.resize_with(size(&v) + 1, || None);
        info!("allocated {} inodes", inodes.len());

        let mut next_id = fuser::FUSE_ROOT_ID;
        // parent inum, inum, value
        let mut worklist: Vec<(u64, u64, Value)> = Vec::new();

        if !(v.is_array() || v.is_object()) {
            panic!(
                "Unable to build a filesystem out of the primitive value '{}'",
                v
            );
        }
        worklist.push((next_id, next_id, v));
        next_id += 1;

        while !worklist.is_empty() {
            let (parent, inum, v) = worklist.pop().unwrap();

            let nl = if config.add_newlines { "\n" } else { "" };
            let entry = match v {
                Value::Null => Entry::File("".into()), // always empty
                Value::Bool(b) => Entry::File(format!("{}{}", b, nl).into_bytes()),
                Value::Number(n) => Entry::File(format!("{}{}", n, nl).into_bytes()),
                Value::String(s) => {
                    let contents = if s.ends_with('\n') { s } else { s + nl };
                    Entry::File(contents.into_bytes())
                }
                Value::Array(vs) => {
                    let mut children = HashMap::new();
                    children.reserve(vs.len());

                    let num_elts = vs.len() as f64;
                    let width = num_elts.log10().ceil() as usize;

                    for (i, child) in vs.into_iter().enumerate() {
                        // TODO 2021-06-08 ability to add prefixes
                        let name = if config.pad_element_names {
                            format!("{:0width$}", i, width = width)
                        } else {
                            format!("{}", i)
                        };

                        children.insert(
                            name,
                            DirEntry {
                                inum: next_id,
                                kind: kind(&child),
                            },
                        );
                        worklist.push((inum, next_id, child));
                        next_id += 1;
                    }

                    Entry::Directory(DirType::List, children)
                }
                Value::Object(fvs) => {
                    let mut children = HashMap::new();
                    children.reserve(fvs.len());

                    for (field, child) in fvs.into_iter() {
                        let original = field.clone();
                        let mut nfield = config.normalize_name(field);

                        while children.contains_key(&nfield) {
                            nfield.push('_');
                        }

                        if original != nfield {
                            info!(
                                "renamed {} to {} (inode {} with parent {})",
                                original, nfield, next_id, parent
                            );
                        }

                        children.insert(
                            nfield,
                            DirEntry {
                                inum: next_id,
                                kind: kind(&child),
                            },
                        );

                        worklist.push((inum, next_id, child));
                        next_id += 1;
                    }

                    Entry::Directory(DirType::Named, children)
                }
            };

            inodes[inum as usize] = Some(Inode {
                parent,
                inum,
                entry,
            });
        }
        assert_eq!(inodes.len() as u64, next_id);
    }

    /// Walks `fs` starting at the inode with number `inum`, producing a JSON
    /// value.
    pub fn value_from_fs(fs: &FS, inum: u64) -> Value {
        match &fs.get(inum).unwrap().entry {
            Entry::File(contents) => {
                // TODO 2021-06-16 better newline handling
                let contents = match String::from_utf8(contents.clone()) {
                    Ok(mut contents) => {
                        if fs.config.add_newlines && contents.ends_with('\n') {
                            contents.truncate(contents.len() - 1);
                        }
                        contents
                    }
                    Err(_) => unimplemented!("binary data JSON serialization"),
                };

                if contents.is_empty() {
                    Value::Null
                } else if contents == "true" {
                    Value::Bool(true)
                } else if contents == "false" {
                    Value::Bool(false)
                } else if let Ok(n) = serde_json::Number::from_str(&contents) {
                    Value::Number(n)
                } else {
                    Value::String(contents)
                }
            }
            Entry::Directory(DirType::List, files) => {
                let mut entries = Vec::with_capacity(files.len());

                let mut files = files.iter().collect::<Vec<_>>();
                files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
                for (_name, DirEntry { inum, .. }) in files.iter() {
                    let v = value_from_fs(fs, *inum);
                    entries.push(v);
                }

                Value::Array(entries)
            }
            Entry::Directory(DirType::Named, files) => {
                let mut entries = serde_json::map::Map::new();

                for (name, DirEntry { inum, .. }) in files.iter() {
                    let v = value_from_fs(fs, *inum);
                    entries.insert(name.into(), v);
                }

                Value::Object(entries)
            }
        }
    }

    /// Predicts filetypes from JSON values.
    ///
    /// `Value::Object` and `Value::Array` map to directories; everything else is a
    /// regular file.
    fn kind(v: &Value) -> FileType {
        match v {
            Value::Object(_) | Value::Array(_) => FileType::Directory,
            _ => FileType::RegularFile,
        }
    }

    /// Calculates the size of a JSON value, i.e., the number of AST nodes used to
    /// represent it. Used for pre-allocating space for inodes in `fs()` below.
    fn size(v: &Value) -> usize {
        match v {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 1,
            Value::Array(vs) => vs.iter().map(|v| size(v)).sum::<usize>() + 1,
            Value::Object(fvs) => fvs.iter().map(|(_, v)| size(v)).sum::<usize>() + 1,
        }
    }
}
