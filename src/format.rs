use std::collections::HashMap;
use std::fs::File;
use std::str::FromStr;

use tracing::{debug, error, info, instrument, warn};

use fuser::FileType;

use super::config::{Config, Input, Output};
use super::fs::{DirEntry, DirType, Entry, Inode, FS};

use ::toml as serde_toml;

/// The possible formats.
///
/// When extending, don't forget to also extend `cli::POSSIBLE_FORMATS`.
#[derive(Copy, Clone, Debug)]
pub enum Format {
    Json,
    Toml,
    Yaml,
}

/// Types classifying string data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Typ {
    Auto,
    Null,
    Boolean,
    Integer,
    Float,
    Datetime,
    String,
    Bytes,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Format::Json => "json",
                Format::Toml => "toml",
                Format::Yaml => "yaml",
            }
        )
    }
}

impl std::fmt::Display for Typ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Typ::Auto => "auto",
                Typ::Null => "null",
                Typ::Boolean => "boolean",
                Typ::Bytes => "bytes",
                Typ::Datetime => "datetime",
                Typ::Float => "float",
                Typ::Integer => "integer",
                Typ::String => "string",
            }
        )
    }
}

#[derive(Debug)]
pub enum ParseFormatError {
    NoSuchFormat(String),
    NoFormatProvided,
}

impl FromStr for Format {
    type Err = ParseFormatError;

    fn from_str(s: &str) -> Result<Self, ParseFormatError> {
        let s = s.trim().to_lowercase();

        if s == "json" {
            Ok(Format::Json)
        } else if s == "toml" {
            Ok(Format::Toml)
        } else if s == "yaml" || s == "yml" {
            Ok(Format::Yaml)
        } else {
            Err(ParseFormatError::NoSuchFormat(s))
        }
    }
}

impl FromStr for Typ {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let s = s.trim().to_lowercase();

        if s == "auto" {
            Ok(Typ::Auto)
        } else if s == "null" {
            Ok(Typ::Null)
        } else if s == "boolean" || s == "bool" {
            Ok(Typ::Boolean)
        } else if s == "bytes" {
            Ok(Typ::Bytes)
        } else if s == "datetime" || s == "date" || s == "time" {
            Ok(Typ::Datetime)
        } else if s == "float" || s == "double" || s == "real" {
            Ok(Typ::Float)
        } else if s == "integer" || s == "int" {
            Ok(Typ::Integer)
        } else if s == "string" {
            Ok(Typ::String)
        } else {
            Err(())
        }
    }
}

impl Format {
    pub fn can_be_pretty(&self) -> bool {
        match self {
            Format::Json | Format::Toml => true,
            Format::Yaml => false,
        }
    }

    /// Generates a filesystem `fs`, reading from `reader` according to a
    /// particular `Config`.
    ///
    /// NB there is no check that `self == fs.config.input_format`!
    #[instrument(level = "info", skip(config))]
    pub fn load(&self, config: Config) -> FS {
        let mut inodes: Vec<Option<Inode>> = Vec::new();

        let reader: Box<dyn std::io::Read> = match &config.input {
            Input::Stdin => Box::new(std::io::stdin()),
            Input::File(file) => {
                let fmt = config.input_format;
                let file = std::fs::File::open(&file).unwrap_or_else(|e| {
                    error!("Unable to open {} for {} input: {}", file.display(), fmt, e);
                    std::process::exit(1);
                });
                Box::new(file)
            }
            Input::Empty => {
                // let's just reserve some space for later and get cracking
                info!("reserving space in empty filesystem");
                inodes.resize_with(1024, || None);

                // create an empty directory
                let contents = HashMap::with_capacity(16);
                inodes[1] = Some(Inode::new(
                    fuser::FUSE_ROOT_ID,
                    fuser::FUSE_ROOT_ID,
                    Entry::Directory(DirType::Named, contents),
                    &config,
                ));
                return FS::new(inodes, config);
            }
        };

        match self {
            Format::Json => {
                info!("reading json value");
                let v: serde_json::Value = serde_json::from_reader(reader).expect("JSON");
                info!("building inodes");
                fs_from_value(v, &config, &mut inodes);
                info!("done");
            }
            Format::Toml => {
                info!("reading toml value");
                let v = toml::from_reader(reader).expect("TOML");
                info!("building inodes");
                fs_from_value(v, &config, &mut inodes);
                info!("done");
            }
            Format::Yaml => {
                info!("reading toml value");
                let v = yaml::from_reader(reader).expect("YAML");
                info!("building inodes");
                fs_from_value(v, &config, &mut inodes);
                info!("done");
            }
        };

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
                let v: serde_json::Value = value_from_fs(fs, fuser::FUSE_ROOT_ID);
                info!("writing");
                debug!("outputting {}", v);
                if fs.config.pretty {
                    serde_json::to_writer_pretty(writer, &v).unwrap();
                } else {
                    serde_json::to_writer(writer, &v).unwrap();
                }
                info!("done")
            }
            Format::Toml => {
                info!("generating toml value");
                let v: serde_toml::Value = value_from_fs(fs, fuser::FUSE_ROOT_ID);
                info!("writing");
                debug!("outputting {}", v);
                if fs.config.pretty {
                    toml::to_writer_pretty(writer, &v).unwrap();
                } else {
                    toml::to_writer(writer, &v).unwrap();
                }
                info!("done");
            }
            Format::Yaml => {
                info!("generating yaml value");
                let v: yaml::Value = value_from_fs(fs, fuser::FUSE_ROOT_ID);
                info!("writing");
                debug!("outputting {}", v);
                yaml::to_writer(writer, &v).unwrap();
                info!("done");
            }
        }
    }
}

enum Node<V> {
    String(Typ, String),
    Bytes(Vec<u8>),

    List(Vec<V>),
    /// We use a `Vec` rather than a `Map` or `HashMap` to ensure we preserve
    /// whatever order during renaming.
    ///
    /// It's a little bit annoying that, e.g., serde_json and toml use different
    /// maps internally. :(
    Map(Vec<(String, V)>),
}

/// Values that can be converted to a `Node`, which can be in turn processed by
/// the worklist algorithm
trait Nodelike
where
    Self: Sized,
{
    /// Number of "nodes" in the given value. This should correspond to the
    /// number of inodes needed to accommodate the value.
    fn size(&self) -> usize;

    /// Predicts filetypes (directory vs. regular file) for values.
    ///
    /// Since FUSE filesystems need to have directories at the root, it's
    /// important that only compound values be converted to fileysstems, i.e.,
    /// values which yield `FileType::Directory`.
    fn kind(&self) -> FileType;

    /// Characterizes the outermost value. Drives the worklist algorithm.
    fn node(self, config: &Config) -> Node<Self>;

    fn from_bytes<T>(v: T, config: &Config) -> Self
    where
        T: AsRef<[u8]>;

    /// Converts from a string.
    ///
    /// Should never be called when `typ == Typ::Bytes`.
    fn from_string(typ: Typ, v: String, config: &Config) -> Self;
    fn from_list_dir(files: Vec<Self>, config: &Config) -> Self;
    fn from_named_dir(files: HashMap<String, Self>, config: &Config) -> Self;
}

/// Given a `Nodelike` value `v`, initializes the vector `inodes` of (nullable)
/// `Inodes` according to a given `config`.
///
/// The current implementation is eager: it preallocates enough inodes and then
/// fills them in using a depth-first traversal.
///
/// Invariant: the index in the vector is the inode number. Inode 0 is invalid,
/// and is left empty.
fn fs_from_value<V>(v: V, config: &Config, inodes: &mut Vec<Option<Inode>>)
where
    V: Nodelike + std::fmt::Display,
{
    // reserve space for everyone else
    // won't work with streaming or lazy generation, but avoids having to resize the vector midway through
    inodes.resize_with(v.size() + 1, || None);
    info!("allocated {} inodes", inodes.len());

    if v.kind() != FileType::Directory {
        error!("The root of the filesystem must be a directory, but '{}' only generates a single file.", v);
        std::process::exit(1);
    }

    let mut next_id = fuser::FUSE_ROOT_ID;
    // parent inum, inum, value
    let mut worklist: Vec<(u64, u64, V)> = vec![(next_id, next_id, v)];

    next_id += 1;

    while !worklist.is_empty() {
        let (parent, inum, v) = worklist.pop().unwrap();

        let entry = match v.node(config) {
            Node::Bytes(b) => Entry::File(Typ::Bytes, b),
            Node::String(t, s) => Entry::File(t, s.into_bytes()),
            Node::List(vs) => {
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
                            kind: child.kind(),
                            original_name: None,
                            inum: next_id,
                        },
                    );
                    worklist.push((inum, next_id, child));
                    next_id += 1;
                }

                Entry::Directory(DirType::List, children)
            }
            Node::Map(fvs) => {
                let mut children = HashMap::new();
                children.reserve(fvs.len());

                for (field, child) in fvs.into_iter() {
                    let original = field.clone();
                    let mut nfield = config.normalize_name(field);

                    // TODO 2021-07-08 could be better to check fvs, but it's a vec now... :/
                    while children.contains_key(&nfield) {
                        nfield.push('_');
                    }

                    let original_name = if original != nfield {
                        info!(
                            "renamed {} to {} (inode {} with parent {})",
                            original, nfield, next_id, parent
                        );
                        Some(original)
                    } else {
                        assert!(config.valid_name(&original));
                        None
                    };

                    children.insert(
                        nfield,
                        DirEntry {
                            kind: child.kind(),
                            original_name,
                            inum: next_id,
                        },
                    );

                    worklist.push((inum, next_id, child));
                    next_id += 1;
                }

                Entry::Directory(DirType::Named, children)
            }
        };

        inodes[inum as usize] = Some(Inode::new(parent, inum, entry, config));
    }
    assert_eq!(inodes.len() as u64, next_id);
}

/// Walks `fs` starting at the inode with number `inum`, producing an
/// appropriate value.
fn value_from_fs<V>(fs: &FS, inum: u64) -> V
where
    V: Nodelike,
{
    match &fs.get(inum).unwrap().entry {
        Entry::File(typ, contents) => {
            // TODO 2021-07-01 use _t to try to force the type
            match String::from_utf8(contents.clone()) {
                Ok(mut contents) if typ != &Typ::Bytes => {
                    if fs.config.add_newlines && contents.ends_with('\n') {
                        contents.truncate(contents.len() - 1);
                    }
                    // TODO 2021-06-24 trim?
                    V::from_string(*typ, contents, &fs.config)
                }
                Ok(_) | Err(_) => V::from_bytes(contents, &fs.config),
            }
        }
        Entry::Directory(DirType::List, files) => {
            let mut entries = Vec::with_capacity(files.len());

            let mut files = files.iter().collect::<Vec<_>>();
            files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
            for (name, DirEntry { inum, .. }) in files.iter() {
                if fs.config.ignored_file(name) {
                    warn!("skipping ignored file '{}'", name);
                    continue;
                }

                let v = value_from_fs(fs, *inum);
                entries.push(v);
            }

            V::from_list_dir(entries, &fs.config)
        }
        Entry::Directory(DirType::Named, files) => {
            let mut entries = HashMap::with_capacity(files.len());

            for (
                name,
                DirEntry {
                    inum,
                    original_name,
                    ..
                },
            ) in files.iter()
            {
                if fs.config.ignored_file(name) {
                    warn!("skipping ignored file '{}'", name);
                    continue;
                }

                let v = value_from_fs(fs, *inum);

                let name = original_name.as_ref().unwrap_or(name).into();
                entries.insert(name, v);
            }

            V::from_named_dir(entries, &fs.config)
        }
    }
}

mod json {
    use super::*;
    use serde_json::Value;

    impl Nodelike for Value {
        /// `Value::Object` and `Value::Array` map to directories; everything else is a
        /// regular file.
        fn kind(&self) -> FileType {
            match self {
                Value::Object(_) | Value::Array(_) => FileType::Directory,
                _ => FileType::RegularFile,
            }
        }

        fn size(&self) -> usize {
            match self {
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 1,
                Value::Array(vs) => vs.iter().map(|v| v.size()).sum::<usize>() + 1,
                Value::Object(fvs) => fvs.iter().map(|(_, v)| v.size()).sum::<usize>() + 1,
            }
        }

        fn node(self, config: &Config) -> Node<Self> {
            let nl = if config.add_newlines { "\n" } else { "" };

            match self {
                Value::Null => Node::String(Typ::Null, "".into()), // always empty
                Value::Bool(b) => Node::String(Typ::Boolean, format!("{}{}", b, nl)),
                Value::Number(n) => Node::String(Typ::Float, format!("{}{}", n, nl)),
                Value::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::decode_config(&s, config.base64) {
                            return Node::Bytes(bytes);
                        }
                    }

                    Node::String(Typ::String, if s.ends_with('\n') { s } else { s + nl })
                }
                Value::Array(vs) => Node::List(vs),
                Value::Object(fvs) => Node::Map(fvs.into_iter().collect()),
            }
        }

        fn from_string(typ: Typ, contents: String, _config: &Config) -> Self {
            match typ {
                Typ::Auto => {
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
                Typ::Boolean => {
                    if contents == "true" {
                        Value::Bool(true)
                    } else if contents == "false" {
                        Value::Bool(false)
                    } else {
                        debug!("string '{}' tagged as boolean", contents);
                        Value::String(contents)
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => Value::String(contents),
                Typ::Float => {
                    if let Ok(n) = serde_json::Number::from_str(&contents) {
                        Value::Number(n)
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value::String(contents)
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = serde_json::Number::from_str(&contents) {
                        Value::Number(n)
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value::String(contents)
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Value::Null
                    } else {
                        debug!("string '{}' tagged as null", contents);
                        Value::String(contents)
                    }
                }
                Typ::String => Value::String(contents),
            }
        }

        fn from_bytes<T>(contents: T, config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value::String(base64::encode_config(contents, config.base64))
        }

        fn from_list_dir(files: Vec<Self>, _config: &Config) -> Self {
            Value::Array(files)
        }

        fn from_named_dir(files: HashMap<String, Self>, _config: &Config) -> Self {
            Value::Object(files.into_iter().collect())
        }
    }
}

mod toml {
    use super::*;
    use serde_toml::Value;

    #[derive(Debug)]
    pub enum Error<E> {
        Io(std::io::Error),
        Toml(E),
    }

    pub fn from_reader(
        mut reader: Box<dyn std::io::Read>,
    ) -> Result<Value, Error<serde_toml::de::Error>> {
        let mut text = String::new();
        let _len = reader.read_to_string(&mut text).map_err(Error::Io)?;
        serde_toml::from_str(&text).map_err(Error::Toml)
    }

    pub fn to_writer(
        mut writer: Box<dyn std::io::Write>,
        v: &Value,
    ) -> Result<(), Error<serde_toml::ser::Error>> {
        let text = serde_toml::to_string(v).map_err(Error::Toml)?;
        writer.write_all(text.as_bytes()).map_err(Error::Io)
    }

    pub fn to_writer_pretty(
        mut writer: Box<dyn std::io::Write>,
        v: &Value,
    ) -> Result<(), Error<serde_toml::ser::Error>> {
        let text = serde_toml::to_string_pretty(v).map_err(Error::Toml)?;
        writer.write_all(text.as_bytes()).map_err(Error::Io)
    }

    impl Nodelike for Value {
        fn kind(&self) -> FileType {
            match self {
                Value::Table(_) | Value::Array(_) => FileType::Directory,
                _ => FileType::RegularFile,
            }
        }

        fn size(&self) -> usize {
            match self {
                Value::Boolean(_)
                | Value::Datetime(_)
                | Value::Float(_)
                | Value::Integer(_)
                | Value::String(_) => 1,
                Value::Array(vs) => vs.iter().map(|v| v.size()).sum::<usize>() + 1,
                Value::Table(fvs) => fvs.iter().map(|(_, v)| v.size()).sum::<usize>() + 1,
            }
        }

        fn node(self, config: &Config) -> Node<Self> {
            let nl = if config.add_newlines { "\n" } else { "" };

            match self {
                Value::Boolean(b) => Node::String(Typ::Boolean, format!("{}{}", b, nl)),
                Value::Datetime(s) => Node::String(Typ::Datetime, s.to_string()),
                Value::Float(n) => Node::String(Typ::Float, format!("{}{}", n, nl)),
                Value::Integer(n) => Node::String(Typ::Integer, format!("{}{}", n, nl)),
                Value::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::decode_config(&s, config.base64) {
                            return Node::Bytes(bytes);
                        }
                    }

                    Node::String(Typ::String, if s.ends_with('\n') { s } else { s + nl })
                }
                Value::Array(vs) => Node::List(vs),
                Value::Table(fvs) => Node::Map(fvs.into_iter().collect()),
            }
        }

        fn from_string(typ: Typ, contents: String, _config: &Config) -> Self {
            match typ {
                Typ::Auto => {
                    if contents == "true" {
                        Value::Boolean(true)
                    } else if contents == "false" {
                        Value::Boolean(false)
                    } else if let Ok(n) = i64::from_str(&contents) {
                        Value::Integer(n)
                    } else if let Ok(n) = f64::from_str(&contents) {
                        Value::Float(n)
                    } else if let Ok(datetime) = str::parse(&contents) {
                        Value::Datetime(datetime)
                    } else {
                        Value::String(contents)
                    }
                }
                Typ::Boolean => {
                    if contents == "true" {
                        Value::Boolean(true)
                    } else if contents == "false" {
                        Value::Boolean(false)
                    } else {
                        debug!("string '{}' tagged as boolean", contents);
                        Value::String(contents)
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => match str::parse(&contents) {
                    Ok(datetime) => Value::Datetime(datetime),
                    Err(e) => {
                        debug!(
                            "string '{}' tagged as datetime, didn't parse: {}",
                            contents, e
                        );
                        Value::String(contents)
                    }
                },
                Typ::Float => {
                    if let Ok(n) = f64::from_str(&contents) {
                        Value::Float(n)
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value::String(contents)
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = i64::from_str(&contents) {
                        Value::Integer(n)
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value::String(contents)
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Value::String(contents)
                    } else {
                        debug!("string '{}' tagged as null", contents);
                        Value::String(contents)
                    }
                }
                Typ::String => Value::String(contents),
            }
        }

        fn from_bytes<T>(contents: T, config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value::String(base64::encode_config(contents, config.base64))
        }

        fn from_list_dir(files: Vec<Self>, _config: &Config) -> Self {
            Value::Array(files)
        }

        fn from_named_dir(files: HashMap<String, Self>, _config: &Config) -> Self {
            Value::Table(files.into_iter().collect())
        }
    }
}

mod yaml {
    use super::*;
    use std::hash::{Hash, Hasher};
    use yaml_rust::{EmitError, ScanError, Yaml};

    #[derive(Clone, Debug)]
    pub struct Value(Yaml);

    #[derive(Debug)]
    pub enum Error<E> {
        Io(std::io::Error),
        Yaml(E),
    }

    pub fn from_reader(mut reader: Box<dyn std::io::Read>) -> Result<Value, Error<ScanError>> {
        let mut text = String::new();
        let _len = reader.read_to_string(&mut text).map_err(Error::Io)?;
        yaml_rust::YamlLoader::load_from_str(&text)
            .map(|vs| {
                Value(if vs.len() == 1 {
                    vs.into_iter().next().unwrap()
                } else {
                    Yaml::Array(vs)
                })
            })
            .map_err(Error::Yaml)
    }

    pub fn to_writer(
        mut writer: Box<dyn std::io::Write>,
        v: &Value,
    ) -> Result<(), Error<EmitError>> {
        let mut text = String::new();
        let mut emitter = yaml_rust::YamlEmitter::new(&mut text);
        emitter.dump(&v.0).map_err(Error::Yaml)?;
        writer.write_all(text.as_bytes()).map_err(Error::Io)
    }

    impl std::fmt::Display for Value {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            let mut emitter = yaml_rust::YamlEmitter::new(f);
            emitter.dump(&self.0).map_err(|e| match e {
                yaml_rust::EmitError::FmtError(e) => e,
                yaml_rust::EmitError::BadHashmapKey => {
                    panic!("unrecoverable YAML display error: BadHashmapKey")
                }
            })
        }
    }

    fn yaml_size(v: &Yaml) -> usize {
        match v {
            Yaml::Real(_)
            | Yaml::Integer(_)
            | Yaml::String(_)
            | Yaml::Boolean(_)
            | Yaml::Null
            | Yaml::BadValue
            | Yaml::Alias(_) => 1,
            Yaml::Array(vs) => vs.iter().map(|v| yaml_size(v)).sum::<usize>() + 1,
            Yaml::Hash(fvs) => fvs.iter().map(|(_, v)| yaml_size(v)).sum::<usize>() + 1,
        }
    }

    fn yaml_key_to_string(v: Yaml) -> String {
        match v {
            Yaml::Boolean(b) => format!("{}", b),
            Yaml::Real(s) => s,
            Yaml::Integer(n) => format!("{}", n),
            Yaml::String(s) => s,
            Yaml::Alias(n) => format!("alias{}", n),
            Yaml::Array(vs) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                vs.hash(&mut hasher);
                format!("{}", hasher.finish())
            }
            Yaml::Hash(fvs) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                fvs.hash(&mut hasher);
                format!("{}", hasher.finish())
            }
            Yaml::Null => "null".into(),
            Yaml::BadValue => "badvalue".into(),
        }
    }

    impl Nodelike for Value {
        fn kind(&self) -> FileType {
            match &self.0 {
                Yaml::Array(_) | Yaml::Hash(_) => FileType::Directory,
                _ => FileType::RegularFile,
            }
        }

        fn size(&self) -> usize {
            yaml_size(&self.0)
        }

        fn node(self, config: &Config) -> Node<Self> {
            let nl = if config.add_newlines { "\n" } else { "" };

            match self.0 {
                Yaml::Null => Node::String(Typ::Null, "".into()),
                Yaml::Boolean(b) => Node::String(Typ::Boolean, format!("{}{}", b, nl)),
                Yaml::Real(s) => Node::String(Typ::Float, s + nl),
                Yaml::Integer(n) => Node::String(Typ::Integer, format!("{}{}", n, nl)),
                Yaml::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::decode_config(&s, config.base64) {
                            return Node::Bytes(bytes);
                        }
                    }

                    Node::String(Typ::String, if s.ends_with('\n') { s } else { s + nl })
                }
                Yaml::Array(vs) => Node::List(vs.into_iter().map(Value).collect()),
                Yaml::Hash(fvs) => Node::Map(
                    fvs.into_iter()
                        .map(|(k, v)| (yaml_key_to_string(k), Value(v)))
                        .collect(),
                ),
                // ??? 2021-06-21 support aliases w/hard links?
                Yaml::Alias(n) => Node::Bytes(format!("alias{}{}", n, nl).into_bytes()),
                Yaml::BadValue => Node::Bytes("bad YAML value".into()),
            }
        }

        fn from_string(typ: Typ, contents: String, _config: &Config) -> Self {
            match typ {
                Typ::Auto => {
                    if contents.is_empty() {
                        Value(Yaml::Null)
                    } else if contents == "true" {
                        Value(Yaml::Boolean(true))
                    } else if contents == "false" {
                        Value(Yaml::Boolean(false))
                    } else if let Ok(n) = i64::from_str(&contents) {
                        Value(Yaml::Integer(n))
                    } else if let Ok(_n) = f64::from_str(&contents) {
                        Value(Yaml::Real(contents))
                    } else {
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Boolean => {
                    if contents == "true" {
                        Value(Yaml::Boolean(true))
                    } else if contents == "false" {
                        Value(Yaml::Boolean(false))
                    } else {
                        debug!("string '{}' tagged as boolean", contents);
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => Value(Yaml::String(contents)),
                Typ::Float => {
                    if let Ok(_n) = f64::from_str(&contents) {
                        Value(Yaml::Real(contents))
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = i64::from_str(&contents) {
                        Value(Yaml::Integer(n))
                    } else {
                        debug!("string '{}' tagged as float", contents);
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Value(Yaml::Null)
                    } else {
                        debug!("string '{}' tagged as null", contents);
                        Value(Yaml::String(contents))
                    }
                }
                Typ::String => Value(Yaml::String(contents)),
            }
        }

        fn from_bytes<T>(contents: T, config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value(Yaml::String(base64::encode_config(contents, config.base64)))
        }

        fn from_list_dir(vs: Vec<Self>, _config: &Config) -> Self {
            Value(Yaml::Array(vs.into_iter().map(|v| v.0).collect()))
        }

        fn from_named_dir(fvs: HashMap<String, Self>, config: &Config) -> Self {
            Value(Yaml::Hash(
                fvs.into_iter()
                    .map(|(k, v)| (Value::from_string(Typ::String, k, config).0, v.0))
                    .collect(),
            ))
        }
    }
}
