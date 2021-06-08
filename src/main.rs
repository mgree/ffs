use clap::{App, Arg};
use serde_json::Value;
use std::collections::HashMap;

fn main() {
    let config = App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file (defaults to '-', meaning STDIN)")
                .default_value("-")
                .index(1),
        )
        .get_matches();

    let input_source = config.value_of("INPUT").expect("input source");

    let reader: Box<dyn std::io::BufRead> = if input_source == "-" {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    } else {
        let file = std::fs::File::open(input_source)
            .unwrap_or_else(|e| panic!("Unable to open {} for JSON input: {}", input_source, e));
        Box::new(std::io::BufReader::new(file))
    };

    let json: Value = serde_json::from_reader(reader).expect("JSON");

    println!("{:?}", json);

    let fs = build_json_fs(json);

    println!("{:?}", fs);
}

#[derive(Debug)]
struct Inode {
    parent: u64,
    entry: Entry,
}

impl Inode {
    pub fn error() -> Self {
        Inode {
            parent: 0,
            entry: Entry::Error("invalid".into()),
        }
    }
}

#[derive(Debug)]
enum Entry {
    File(String),
    NamedDirectory(HashMap<String, u64>),
    ListDirectory(Vec<u64>),
    Error(String),
}

fn build_json_fs(v: Value) -> Vec<Inode> {
    let mut inodes: Vec<Inode> = Vec::new();
    // get zero-indexing for free, with a nice non-zero check to boot
    inodes.push(Inode {
        parent: 0,
        entry: Entry::Error("inode 0 is invalid".into()),
    });
    // TODO 2021-06-07 reserve based on guess of size

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

        let entry = match v {
            Value::Null => Entry::File("".into()),
            Value::Bool(b) => Entry::File(format!("{}", b)),
            Value::Number(n) => Entry::File(format!("{}", n)),
            Value::String(s) => Entry::File(s),
            Value::Array(vs) => {
                let mut children = Vec::new();
                children.reserve(vs.len());

                for child in vs.into_iter() {
                    worklist.push((inum, next_id, child));
                    children.push(next_id);
                    next_id += 1;
                }

                Entry::ListDirectory(children)
            }
            Value::Object(fvs) => {
                let mut children = HashMap::new();
                children.reserve(fvs.len());

                for (field, child) in fvs.into_iter() {
                    worklist.push((inum, next_id, child));
                    children.insert(field, next_id);
                    next_id += 1;
                }

                Entry::NamedDirectory(children)
            }
        };

        let idx = inum as usize;
        if idx >= inodes.len() {
            inodes.resize_with(idx + 1, || Inode::error());
        }
        inodes[idx] = Inode { parent, entry };
    }
    assert_eq!(inodes.len() as u64, next_id);

    inodes
}
