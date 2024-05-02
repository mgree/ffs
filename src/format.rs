use std::collections::BTreeMap;
use std::str::FromStr;

use tracing::debug;

use fuser::FileType;

use super::config::Config;

use ::toml as serde_toml;

#[macro_export]
macro_rules! time_ns {
    ($msg:expr, $e:expr, $timing:expr) => {{
        let start = std::time::Instant::now();
        let v = $e;

        let msg = $msg;
        let elapsed = start.elapsed().as_nanos();
        if $timing {
            eprintln!("{msg},{elapsed}");
        } else {
            info!("{msg} ({elapsed}ns)");
        }
        v
    }};
}

/// The possible formats.
///
/// When extending, don't forget to also extend `cli::POSSIBLE_FORMATS`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
}

/// The ffs data model; it represents just one layer---lists and maps are
/// parameterized over the underlying value type V.
pub enum Node<V> {
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
pub trait Nodelike
where
    Self: Clone + std::fmt::Debug + Default + std::fmt::Display + Sized,
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
    fn from_named_dir(files: BTreeMap<String, Self>, config: &Config) -> Self;

    /// Loading
    fn from_reader(reader: Box<dyn std::io::Read>) -> Self;

    /// Saving, with optional pretty printing
    fn to_writer(&self, writer: Box<dyn std::io::Write>, pretty: bool);
}

////////////////////////////////////////////////////////////////////////////////
/// JSON Nodelike implementation
pub mod json {
    use super::*;
    use base64::Engine as _;
    pub use serde_json::Value;

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
                Value::Bool(b) => Node::String(Typ::Boolean, format!("{b}{nl}")),
                Value::Number(n) => Node::String(Typ::Float, format!("{n}{nl}")),
                Value::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&s) {
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
                        debug!("string '{contents}' tagged as boolean");
                        Value::String(contents)
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => Value::String(contents),
                Typ::Float => {
                    if let Ok(n) = serde_json::Number::from_str(&contents) {
                        Value::Number(n)
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Value::String(contents)
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = serde_json::Number::from_str(&contents) {
                        Value::Number(n)
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Value::String(contents)
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Value::Null
                    } else {
                        debug!("string '{contents}' tagged as null");
                        Value::String(contents)
                    }
                }
                Typ::String => Value::String(contents),
            }
        }

        fn from_bytes<T>(contents: T, _config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value::String(base64::engine::general_purpose::STANDARD.encode(contents))
        }

        fn from_list_dir(files: Vec<Self>, _config: &Config) -> Self {
            Value::Array(files)
        }

        fn from_named_dir(files: BTreeMap<String, Self>, _config: &Config) -> Self {
            Value::Object(files.into_iter().collect())
        }

        fn to_writer(&self, writer: Box<dyn std::io::Write>, pretty: bool) {
            if pretty {
                serde_json::to_writer_pretty(writer, self).unwrap();
            } else {
                serde_json::to_writer(writer, self).unwrap();
            }
        }
        fn from_reader(reader: std::boxed::Box<dyn std::io::Read>) -> Self {
            serde_json::from_reader(reader).expect("JSON")
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// TOML Nodelike implementation
pub mod toml {
    use super::*;
    use base64::Engine;
    use serde_toml::Value as Toml;

    #[derive(Clone, Debug)]
    pub struct Value(serde_toml::Value);

    impl std::fmt::Display for Value {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            self.0.fmt(f)
        }
    }

    impl Default for Value {
        fn default() -> Self {
            Value(Toml::String("".into()))
        }
    }

    fn toml_size(v: &Toml) -> usize {
        match v {
            Toml::Boolean(_)
            | Toml::Datetime(_)
            | Toml::Float(_)
            | Toml::Integer(_)
            | Toml::String(_) => 1,
            Toml::Array(vs) => vs.iter().map(toml_size).sum::<usize>() + 1,
            Toml::Table(fvs) => fvs.iter().map(|(_, v)| toml_size(v)).sum::<usize>() + 1,
        }
    }

    impl Nodelike for Value {
        fn kind(&self) -> FileType {
            match self.0 {
                Toml::Table(_) | Toml::Array(_) => FileType::Directory,
                _ => FileType::RegularFile,
            }
        }

        fn size(&self) -> usize {
            toml_size(&self.0)
        }

        fn node(self, config: &Config) -> Node<Self> {
            let nl = if config.add_newlines { "\n" } else { "" };

            match self.0 {
                Toml::Boolean(b) => Node::String(Typ::Boolean, format!("{b}{nl}")),
                Toml::Datetime(s) => Node::String(Typ::Datetime, s.to_string()),
                Toml::Float(n) => Node::String(Typ::Float, format!("{n}{nl}")),
                Toml::Integer(n) => Node::String(Typ::Integer, format!("{n}{nl}")),
                Toml::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&s) {
                            return Node::Bytes(bytes);
                        }
                    }

                    Node::String(Typ::String, if s.ends_with('\n') { s } else { s + nl })
                }
                Toml::Array(vs) => Node::List(vs.into_iter().map(Value).collect()),
                Toml::Table(fvs) => {
                    Node::Map(fvs.into_iter().map(|(f, v)| (f, Value(v))).collect())
                }
            }
        }

        fn from_string(typ: Typ, contents: String, _config: &Config) -> Self {
            let v = match typ {
                Typ::Auto => {
                    if contents == "true" {
                        Toml::Boolean(true)
                    } else if contents == "false" {
                        Toml::Boolean(false)
                    } else if let Ok(n) = i64::from_str(&contents) {
                        Toml::Integer(n)
                    } else if let Ok(n) = f64::from_str(&contents) {
                        Toml::Float(n)
                    } else if let Ok(datetime) = str::parse(&contents) {
                        Toml::Datetime(datetime)
                    } else {
                        Toml::String(contents)
                    }
                }
                Typ::Boolean => {
                    if contents == "true" {
                        Toml::Boolean(true)
                    } else if contents == "false" {
                        Toml::Boolean(false)
                    } else {
                        debug!("string '{contents}' tagged as boolean");
                        Toml::String(contents)
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => match str::parse(&contents) {
                    Ok(datetime) => Toml::Datetime(datetime),
                    Err(e) => {
                        debug!("string '{contents}' tagged as datetime, didn't parse: {e}");
                        Toml::String(contents)
                    }
                },
                Typ::Float => {
                    if let Ok(n) = f64::from_str(&contents) {
                        Toml::Float(n)
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Toml::String(contents)
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = i64::from_str(&contents) {
                        Toml::Integer(n)
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Toml::String(contents)
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Toml::String(contents)
                    } else {
                        debug!("string '{contents}' tagged as null");
                        Toml::String(contents)
                    }
                }
                Typ::String => Toml::String(contents),
            };

            Value(v)
        }

        fn from_bytes<T>(contents: T, _config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value(Toml::String(
                base64::engine::general_purpose::STANDARD.encode(contents),
            ))
        }

        fn from_list_dir(files: Vec<Self>, _config: &Config) -> Self {
            Value(Toml::Array(files.into_iter().map(|v| v.0).collect()))
        }

        fn from_named_dir(files: BTreeMap<String, Self>, _config: &Config) -> Self {
            Value(Toml::Table(
                files.into_iter().map(|(f, v)| (f, v.0)).collect(),
            ))
        }

        fn from_reader(mut reader: Box<dyn std::io::Read>) -> Self {
            let mut text = String::new();
            let _len = reader.read_to_string(&mut text).unwrap();
            Value(serde_toml::from_str(&text).expect("TOML"))
        }

        fn to_writer(&self, mut writer: Box<dyn std::io::Write>, pretty: bool) {
            let text = if pretty {
                serde_toml::to_string_pretty(&self.0).unwrap()
            } else {
                serde_toml::to_string(&self.0).unwrap()
            };
            writer.write_all(text.as_bytes()).unwrap();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// YAML Nodelike implementation
pub mod yaml {
    use super::*;
    use base64::Engine;
    use std::hash::{Hash, Hasher};
    use yaml_rust::Yaml;

    #[derive(Clone, Debug)]
    pub struct Value(Yaml);

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

    impl Default for Value {
        fn default() -> Self {
            Value(Yaml::Null)
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
            Yaml::Array(vs) => vs.iter().map(yaml_size).sum::<usize>() + 1,
            Yaml::Hash(fvs) => fvs.iter().map(|(_, v)| yaml_size(v)).sum::<usize>() + 1,
        }
    }

    fn yaml_key_to_string(v: Yaml) -> String {
        match v {
            Yaml::Boolean(b) => format!("{b}"),
            Yaml::Real(s) => s,
            Yaml::Integer(n) => format!("{n}"),
            Yaml::String(s) => s,
            Yaml::Alias(n) => format!("alias{n}"),
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
                Yaml::Boolean(b) => Node::String(Typ::Boolean, format!("{b}{nl}")),
                Yaml::Real(s) => Node::String(Typ::Float, s + nl),
                Yaml::Integer(n) => Node::String(Typ::Integer, format!("{n}{nl}")),
                Yaml::String(s) => {
                    if config.try_decode_base64 {
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&s) {
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
                Yaml::Alias(n) => Node::Bytes(format!("alias{n}{nl}").into_bytes()),
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
                        debug!("string '{contents}' tagged as boolean");
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Bytes => panic!("from_string called at typ::bytes"),
                Typ::Datetime => Value(Yaml::String(contents)),
                Typ::Float => {
                    if let Ok(_n) = f64::from_str(&contents) {
                        Value(Yaml::Real(contents))
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Integer => {
                    if let Ok(n) = i64::from_str(&contents) {
                        Value(Yaml::Integer(n))
                    } else {
                        debug!("string '{contents}' tagged as float");
                        Value(Yaml::String(contents))
                    }
                }
                Typ::Null => {
                    if contents.is_empty() {
                        Value(Yaml::Null)
                    } else {
                        debug!("string '{contents}' tagged as null");
                        Value(Yaml::String(contents))
                    }
                }
                Typ::String => Value(Yaml::String(contents)),
            }
        }

        fn from_bytes<T>(contents: T, _config: &Config) -> Self
        where
            T: AsRef<[u8]>,
        {
            Value(Yaml::String(
                base64::engine::general_purpose::STANDARD.encode(contents),
            ))
        }

        fn from_list_dir(vs: Vec<Self>, _config: &Config) -> Self {
            Value(Yaml::Array(vs.into_iter().map(|v| v.0).collect()))
        }

        fn from_named_dir(fvs: BTreeMap<String, Self>, config: &Config) -> Self {
            Value(Yaml::Hash(
                fvs.into_iter()
                    .map(|(k, v)| (Value::from_string(Typ::String, k, config).0, v.0))
                    .collect(),
            ))
        }

        fn from_reader(mut reader: Box<dyn std::io::Read>) -> Self {
            let mut text = String::new();
            let _len = reader.read_to_string(&mut text).unwrap();
            yaml_rust::YamlLoader::load_from_str(&text)
                .map(|vs| {
                    Value(if vs.len() == 1 {
                        vs.into_iter().next().unwrap()
                    } else {
                        Yaml::Array(vs)
                    })
                })
                .expect("YAML")
        }

        fn to_writer(&self, mut writer: Box<dyn std::io::Write>, _pretty: bool) {
            let mut text = String::new();
            let mut emitter = yaml_rust::YamlEmitter::new(&mut text);
            emitter.dump(&self.0).unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
    }
}
