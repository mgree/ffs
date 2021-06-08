use clap::{App, Arg};
use fuser::FileAttr;
use fuser::FileType;
use fuser::Filesystem;
use fuser::MountOption;
use fuser::ReplyAttr;
use fuser::ReplyData;
use fuser::ReplyDirectory;
use fuser::ReplyEntry;
use fuser::Request;
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

fn main() {
    let config = App::new("ffs")
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

    let autounmount = config.is_present("AUTOUNMOUNT");

    let mount_point = Path::new(config.value_of("MOUNT").expect("mount point"));
    if !mount_point.exists() {
        panic!("Mount point {} does not exist.", mount_point.display());
    }

    let input_source = config.value_of("INPUT").expect("input source");

    let reader: Box<dyn std::io::BufRead> = if input_source == "-" {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    } else {
        let file = std::fs::File::open(input_source)
            .unwrap_or_else(|e| panic!("Unable to open {} for JSON input: {}", input_source, e));
        Box::new(std::io::BufReader::new(file))
    };

    let json: Value = serde_json::from_reader(reader).expect("JSON");
    let fs = FS::from(json);

    let mut options = vec![MountOption::RO, MountOption::FSName(input_source.into())];
    if autounmount {
        options.push(MountOption::AutoUnmount);
    }
    fuser::mount2(fs, mount_point, &options).unwrap();
}

#[derive(Debug)]
struct FS {
    inodes: Vec<Option<Inode>>,
    timestamp: std::time::SystemTime,
}

const TTL: Duration = Duration::from_secs(300);

#[derive(Debug)]
struct Inode {
    parent: u64,
    inum: u64,
    entry: Entry,
}

#[derive(Debug)]
enum Entry {
    File(String),
    NamedDirectory(HashMap<String, DirEntry>),
    ListDirectory(Vec<DirEntry>),
}

#[derive(Debug)]
struct DirEntry {
    kind: FileType,
    inum: u64,
}

#[derive(Debug)]
enum FSError {
    NoSuchInode(u64),
    InvalidInode(u64),
}

impl FS {
    fn get(&self, inum: u64) -> Result<&Inode, FSError> {
        let idx = inum as usize;

        if idx >= self.inodes.len() {
            return Err(FSError::NoSuchInode(inum));
        }

        match &self.inodes[idx] {
            None => Err(FSError::InvalidInode(inum)),
            Some(inode) => Ok(inode),
        }
    }

    pub fn attr(&self, inode: &Inode) -> FileAttr {
        let size = inode.entry.size();
        let kind = inode.entry.kind();

        let perm = if kind == FileType::Directory {
            0o755
        } else {
            0o644
        };

        FileAttr {
            ino: inode.inum,
            atime: self.timestamp,
            crtime: self.timestamp,
            ctime: self.timestamp,
            mtime: self.timestamp,
            nlink: 1,
            size,
            blksize: 1,
            blocks: size,
            kind,
            // TODO 2021-07-07 getpwnam upfront, store in fs
            uid: 501, // first user on macOS
            gid: 20,  // staff on macOS
            perm,
            rdev: 0,
            padding: 0,
            flags: 0, // weird macOS thing
        }
    }
}

impl Entry {
    pub fn size(&self) -> u64 {
        match self {
            Entry::File(s) => s.len() as u64,
            Entry::NamedDirectory(files) => {
                files.iter().map(|(name, _inum)| name.len() as u64).sum()
            }
            Entry::ListDirectory(elts) => elts.len() as u64,
        }
    }

    pub fn kind(&self) -> FileType {
        match self {
            Entry::File(_) => FileType::RegularFile,
            Entry::NamedDirectory(_) | Entry::ListDirectory(_) => FileType::Directory,
        }
    }
}

impl Filesystem for FS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let dir = match self.get(parent) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        let filename = match name.to_str() {
            None => {
                reply.error(libc::ENOENT);
                return;
            }
            Some(name) => name,
        };

        match &dir.entry {
            Entry::NamedDirectory(files) => match files.get(filename) {
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
                Some(DirEntry { inum, .. }) => {
                    let file = match self.get(*inum) {
                        Err(_e) => {
                            reply.error(libc::ENOENT);
                            return;
                        }
                        Ok(inode) => inode,
                    };

                    reply.entry(&TTL, &self.attr(file), 0);
                    return;
                }
            },
            Entry::ListDirectory(_elts) => unimplemented!("TODO 2021-07-07 list directories"),
            _ => {
                reply.error(libc::ENOTDIR);
                return;
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        reply.attr(&TTL, &self.attr(file));
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        match &file.entry {
            Entry::File(s) => reply.data(&s.as_bytes()[offset as usize..]),
            _ => reply.error(libc::ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inode = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        match &inode.entry {
            Entry::File(_) => reply.error(libc::ENOTDIR),
            Entry::ListDirectory(_elts) => unimplemented!("TODO 2021-06-07 list directory readdir"),
            Entry::NamedDirectory(files) => {
                let dot_entries = vec![
                    (ino, FileType::Directory, "."),
                    (inode.parent, FileType::Directory, ".."),
                ];

                let entries = files
                    .into_iter()
                    .map(|(filename, DirEntry { inum, kind })| (*inum, *kind, filename.as_str()));

                for (i, entry) in dot_entries
                    .into_iter()
                    .chain(entries)
                    .into_iter()
                    .enumerate()
                    .skip(offset as usize)
                {
                    if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                        break;
                    }
                }
                reply.ok()
            }
        }
    }
}

fn kind(v: &Value) -> FileType {
    match v {
        Value::Object(_) | Value::Array(_) => FileType::Directory,
        _ => FileType::RegularFile,
    }
}

impl From<Value> for FS {
    fn from(v: Value) -> Self {
        let mut inodes: Vec<Option<Inode>> = Vec::new();
        // get zero-indexing for free, with a nice non-zero check to boot
        inodes.push(None);
        // TODO 2021-06-07 reserve based on guess or calculated size

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
                        children.push(DirEntry {
                            inum: next_id,
                            kind: kind(&child),
                        });
                        worklist.push((inum, next_id, child));
                        next_id += 1;
                    }

                    Entry::ListDirectory(children)
                }
                Value::Object(fvs) => {
                    let mut children = HashMap::new();
                    children.reserve(fvs.len());

                    for (field, child) in fvs.into_iter() {
                        children.insert(
                            field,
                            DirEntry {
                                inum: next_id,
                                kind: kind(&child),
                            },
                        );
                        worklist.push((inum, next_id, child));
                        // TODO 2021-06-07 check field name validity!
                        next_id += 1;
                    }

                    Entry::NamedDirectory(children)
                }
            };

            let idx = inum as usize;
            if idx >= inodes.len() {
                inodes.resize_with(idx + 1, || None);
            }
            inodes[idx] = Some(Inode {
                parent,
                inum,
                entry,
            });
        }
        assert_eq!(inodes.len() as u64, next_id);

        FS {
            inodes,
            timestamp: std::time::SystemTime::now(),
        }
    }
}
