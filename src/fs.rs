use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::Duration;

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};

use tracing::{warn, debug};

use super::config::Config;

/// A filesystem `FS` is just a vector of nullable inodes, where the index is
/// the inode number.
///
/// NB that inode 0 is always invalid.
#[derive(Debug)]
pub struct FS {
    /// Vector of nullable inodes; the index is the inode number.
    pub inodes: Vec<Option<Inode>>,
    /// Configuration, which determines various file attributes.
    pub config: Config,
}

/// Default TTL on information passed to the OS, which caches responses.
const TTL: Duration = Duration::from_secs(300);

/// An inode, the core structure in the filesystem.
#[derive(Debug)]
pub struct Inode {
    pub parent: u64,
    pub inum: u64,
    pub entry: Entry,
}

#[derive(Debug)]
pub enum Entry {
    File(String),
    Directory(DirType, HashMap<String, DirEntry>),
}

#[derive(Debug)]
pub struct DirEntry {
    pub kind: FileType,
    pub inum: u64,
}

#[derive(Debug)]
pub enum DirType {
    Named,
    List,
}

#[derive(Debug)]
pub enum FSError {
    NoSuchInode(u64),
    InvalidInode(u64),
}

impl FS {
    fn fresh_inode(&mut self, parent: u64, entry: Entry) -> u64 {
        let inum = self.inodes.len() as u64;

        self.inodes.push(Some(Inode {
            parent,
            inum,
            entry,
        }));

        inum
    }

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

    fn get_mut(&mut self, inum: u64) -> Result<&mut Inode, FSError> {
        let idx = inum as usize;

        if idx >= self.inodes.len() {
            return Err(FSError::NoSuchInode(inum));
        }

        match self.inodes.get_mut(idx) {
            Some(Some(inode)) => Ok(inode),
            _ => Err(FSError::InvalidInode(inum)),
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

        let nlink: u32 = match &inode.entry {
            Entry::Directory(_, files) => {
                2 + files
                    .iter()
                    .filter(|(_, de)| de.kind == FileType::Directory)
                    .count() as u32
            }
            Entry::File(_) => 1,
        };

        FileAttr {
            ino: inode.inum,
            atime: self.config.timestamp,
            crtime: self.config.timestamp,
            ctime: self.config.timestamp,
            mtime: self.config.timestamp,
            nlink,
            size,
            blksize: 1,
            blocks: size,
            kind,
            uid: self.config.uid,
            gid: self.config.gid,
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
            Entry::Directory(DirType::Named, files) => {
                files.iter().map(|(name, _inum)| name.len() as u64).sum()
            }
            Entry::Directory(DirType::List, files) => files.len() as u64,
        }
    }

    pub fn kind(&self) -> FileType {
        match self {
            Entry::File(_) => FileType::RegularFile,
            Entry::Directory(..) => FileType::Directory,
        }
    }
}

impl Filesystem for FS {
    fn destroy(&mut self, _req: &Request) {
        debug!("{:?}", self);
    }

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
            Entry::Directory(_kind, files) => match files.get(filename) {
                None => {
                    reply.error(libc::ENOENT);
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
                }
            },
            _ => {
                reply.error(libc::ENOTDIR);
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
            Entry::Directory(_kind, files) => {
                let dot_entries = vec![
                    (ino, FileType::Directory, "."),
                    (inode.parent, FileType::Directory, ".."),
                ];

                let entries = files
                    .iter()
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

    fn mknod(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        // access control
        if req.uid() != self.config.uid || req.gid() != self.config.gid {
            reply.error(libc::EACCES);
            return;
        }

        // make sure we have a good file type
        let file_type = mode & libc::S_IFMT as u32;
        if !vec![libc::S_IFREG as u32, libc::S_IFDIR as u32].contains(&file_type) {
            warn!(
                "mknod only supports regular files and directories; got {:o}",
                mode
            );
            reply.error(libc::ENOSYS);
            return;
        }

        // get the filename
        let filename = match name.to_str() {
            None => {
                reply.error(libc::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure the parent exists, is a directory, and doesn't have that file
        match self.get(parent) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => match &inode.entry {
                Entry::File(_) => {
                    reply.error(libc::ENOTDIR);
                    return;
                }
                Entry::Directory(_dirtype, files) => {
                    if files.contains_key(filename) {
                        reply.error(libc::EEXIST);
                        return;
                    }
                }
            },
        };

        // create the inode entry
        let (entry, kind) = if file_type == libc::S_IFREG as u32 {
            (Entry::File(String::new()), FileType::RegularFile)
        } else {
            assert_eq!(file_type, libc::S_IFDIR as u32);
            (
                Entry::Directory(DirType::Named, HashMap::new()),
                FileType::Directory,
            )
        };

        // allocate the inode
        let inum = self.fresh_inode(parent, entry);

        // update the parent
        // NB we can't get_mut the parent earlier due to borrowing restrictions
        match self.get_mut(parent) {
            Err(_e) => unreachable!("error finding parent again"),
            Ok(inode) => match &mut inode.entry {
                Entry::File(_) => unreachable!("parent changed to a regular file"),
                Entry::Directory(_dirtype, files) => {
                    files.insert(filename.into(), DirEntry { kind, inum });
                }
            },
        };

        reply.entry(&TTL, &self.attr(self.get(inum).unwrap()), 0);

    }
}
