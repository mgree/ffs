use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, SystemTime};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

#[cfg(target_os = "macos")]
use fuser::ReplyXTimes;

use tracing::{debug, info, instrument, warn};

use super::config::{Config, Output};

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
    /// Dirty bit: set to `true` when there are outstanding writes
    dirty: Cell<bool>,
    /// Synced bit: set to `true` if syncing has _ever_ happened
    synced: Cell<bool>,
}

/// Default TTL on information passed to the OS, which caches responses.
const TTL: Duration = Duration::from_secs(300);

/// An inode, the core structure in the filesystem.
#[derive(Debug)]
pub struct Inode {
    pub parent: u64,
    pub inum: u64,
    pub mode: u16,
    pub entry: Entry,
}

#[derive(Debug)]
pub enum Entry {
    // TODO 2021-06-14 need a 'written' flag to determine whether or not to
    // strip newlines during writeback
    File(Vec<u8>),
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
    pub fn new(inodes: Vec<Option<Inode>>, config: Config) -> Self {
        FS {
            inodes,
            config,
            dirty: Cell::new(false),
            synced: Cell::new(false),
        }
    }

    fn fresh_inode(&mut self, parent: u64, entry: Entry, mode: u32) -> u64 {
        self.dirty.set(true);

        let inum = self.inodes.len() as u64;
        let mode = (mode & 0o777) as u16;

        self.inodes
            .push(Some(Inode::with_mode(parent, inum, entry, mode)));

        inum
    }

    fn check_access(&self, req: &Request) -> bool {
        req.uid() == self.config.uid
    }

    pub fn get(&self, inum: u64) -> Result<&Inode, FSError> {
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

    /// Gets the `FileAttr` of a given `Inode`. Much of this is computed each
    /// time: the size, the kind, permissions, and number of hard links.
    pub fn attr(&self, inode: &Inode) -> FileAttr {
        let size = inode.entry.size();
        let kind = inode.entry.kind();

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
            perm: inode.mode,
            rdev: 0,
            flags: 0, // weird macOS thing
        }
    }

    /// Tries to synchronize the in-memory `FS` with its on-disk representation.
    ///
    /// Depending on output conventions and the state of the `FS`, nothing may
    /// happen. In particular:
    ///
    ///   - if a sync has happened before and the `FS` isn't dirty, nothing will
    ///     happen (to prevent pointless writes)
    ///
    ///   - if `self.config.output == Output::Stdout` and `last_sync == false`,
    ///     nothing will happen (to prevent redundant writes to STDOUT)
    #[instrument(level = "debug", skip(self), fields(synced = self.dirty.get(), dirty = self.dirty.get()))]
    pub fn sync(&self, last_sync: bool) {
        info!("called");
        debug!("{:?}", self.inodes);

        if self.synced.get() && !self.dirty.get() {
            info!("skipping sync; already synced and not dirty");
            return;
        }

        match self.config.output {
            Output::Stdout if !last_sync => {
                info!("skipping sync; not last sync, using stdout");
                return;
            }
            _ => (),
        };

        self.config.output_format.save(self);
        self.dirty.set(false);
        self.synced.set(true);
    }
}

impl Inode {
    pub fn new(parent: u64, inum: u64, entry: Entry, config: &Config) -> Self {
        let mode = config.mode(entry.kind());
        Inode::with_mode(parent, inum, entry, mode)
    }

    pub fn with_mode(parent: u64, inum: u64, entry: Entry, mode: u16) -> Self {
        Inode {
            parent,
            inum,
            mode,
            entry,
        }
    }
}

impl Entry {
    /// Computes the size of an entry
    ///
    /// Files are simply their length (not capacity)
    ///
    /// Directory size is informed by the object model:
    ///
    ///   - `DirType::List` directories are only their length (since names won't
    ///     matter)
    ///   - `DirType::Named` directories are the sum of the length of the
    ///     filenames
    pub fn size(&self) -> u64 {
        match self {
            Entry::File(s) => s.len() as u64,
            Entry::Directory(DirType::Named, files) => {
                files.iter().map(|(name, _inum)| name.len() as u64).sum()
            }
            Entry::Directory(DirType::List, files) => files.len() as u64,
        }
    }

    /// Determines the `FileType` of an `Entry`
    pub fn kind(&self) -> FileType {
        match self {
            Entry::File(_) => FileType::RegularFile,
            Entry::Directory(..) => FileType::Directory,
        }
    }
}

impl Drop for FS {
    /// Synchronizes the `FS`, calling `FS::sync` with `last_sync == true`.
    #[instrument(level = "debug", skip(self), fields(dirty = self.dirty.get()))]
    fn drop(&mut self) {
        self.sync(true); // last sync
    }
}

impl Filesystem for FS {
    #[instrument(level = "debug", skip(self, _req), fields(dirty = self.dirty.get()))]
    fn destroy(&mut self, _req: &Request) {
        info!("called");
        // It WOULD make sense to call `sync` here, but this function doesn't
        // seem to be called on Linux... so we call `self.sync(true)` in
        // `Drop::drop`, instead.
        //
        // See https://github.com/cberner/fuser/issues/153
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        info!("called");
        reply.statfs(0, 0, 0, 0, 0, 1, 255, 0);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn access(&mut self, req: &Request, inode: u64, mut mask: i32, reply: ReplyEmpty) {
        info!("called");
        if mask == libc::F_OK {
            reply.ok();
            return;
        }

        match self.get(inode) {
            Ok(inode) => {
                // cribbed from https://github.com/cberner/fuser/blob/4639a490f4aa7dfe8a342069a761d4cf2bd8f821/examples/simple.rs#L1703-L1736
                let attr = self.attr(inode);
                let mode = attr.perm as i32;

                if req.uid() == 0 {
                    // root only allowed to exec if one of the X bits is set
                    mask &= libc::X_OK;
                    mask -= mask & (mode >> 6);
                    mask -= mask & (mode >> 3);
                    mask -= mask & mode;
                } else if req.uid() == self.config.uid {
                    mask -= mask & (mode >> 6);
                } else if req.gid() == self.config.gid {
                    mask -= mask & (mode >> 3);
                } else {
                    mask -= mask & mode;
                }

                if mask == 0 {
                    reply.ok();
                } else {
                    reply.error(libc::EACCES);
                }
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        info!("called");
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

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        info!("called");
        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        reply.attr(&TTL, &self.attr(file));
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn setattr(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        info!("called");

        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        if let Some(mode) = mode {
            debug!("chmod() called with {:?}, {:o}", ino, mode);

            if mode != mode & 0o777 {
                info!("truncating mode {:o} to {:o}", mode, mode & 0o777);
            }
            let mode = (mode as u16) & 0o777;

            let mut attrs = self.attr(file);
            if req.uid() != 0 && req.uid() != attrs.uid {
                reply.error(libc::EPERM);
                return;
            }

            attrs.perm = mode;
            reply.attr(&Duration::new(0, 0), &attrs);

            self.get_mut(ino).unwrap().mode = mode;
            return;
        }

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
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
        info!("called");
        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        match &file.entry {
            Entry::File(s) => reply.data(&s[offset as usize..]),
            _ => reply.error(libc::ENOENT),
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        info!("called");

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

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn create(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        info!("called");

        // force the system to use mknod and open
        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
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
        info!("called");

        // access control
        if !self.check_access(req) {
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
            (Entry::File(Vec::new()), FileType::RegularFile)
        } else {
            assert_eq!(file_type, libc::S_IFDIR as u32);
            (
                Entry::Directory(DirType::Named, HashMap::new()),
                FileType::Directory,
            )
        };

        // allocate the inode (sets dirty bit)
        let inum = self.fresh_inode(parent, entry, mode);

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

        assert!(self.dirty.get());
        reply.entry(&TTL, &self.attr(self.get(inum).unwrap()), 0);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn mkdir(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        info!("called");

        if !self.check_access(req) {
            reply.error(libc::EACCES);
            return;
        }

        // get the new directory name
        let filename = match name.to_str() {
            None => {
                reply.error(libc::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure the parent exists, is a directory, and doesn't have anything with that name
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
        let entry = Entry::Directory(DirType::Named, HashMap::new());
        let kind = FileType::Directory;

        // allocate the inode (sets dirty bit)
        let inum = self.fresh_inode(parent, entry, mode);

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

        assert!(self.dirty.get());
        reply.entry(&TTL, &self.attr(self.get(inum).unwrap()), 0);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn write(
        &mut self,
        req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        info!("called");

        assert!(offset >= 0);

        // access control
        if !self.check_access(req) {
            reply.error(libc::EACCES);
            return;
        }

        // find inode
        let file = match self.get_mut(ino) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        // load contents
        let contents = match &mut file.entry {
            Entry::File(contents) => contents,
            Entry::Directory(_, _) => {
                reply.error(libc::EISDIR);
                return;
            }
        };

        // make space
        let extra_bytes = (offset + data.len() as i64) - contents.len() as i64;
        if extra_bytes > 0 {
            contents.resize(contents.len() + extra_bytes as usize, 0);
        }

        // actually write
        let offset = offset as usize;
        contents[offset..offset + data.len()].copy_from_slice(data);
        self.dirty.set(true);

        reply.written(data.len() as u32);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        // access control
        if !self.check_access(req) {
            reply.error(libc::EACCES);
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

        // find the parent
        let files = match self.get_mut(parent) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(Inode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(Inode {
                entry: Entry::File(_),
                ..
            }) => {
                reply.error(libc::ENOTDIR);
                return;
            }
        };

        // ensure it's a regular file
        match files.get(filename) {
            Some(DirEntry {
                kind: FileType::RegularFile,
                ..
            }) => (),
            _ => {
                reply.error(libc::EPERM);
                return;
            }
        }

        // try to remove it
        let res = files.remove(filename);
        assert!(res.is_some());
        self.dirty.set(true);
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn rmdir(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        // access control
        if !self.check_access(req) {
            reply.error(libc::EACCES);
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

        // find the parent
        let files = match self.get(parent) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(Inode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(Inode {
                entry: Entry::File(_),
                ..
            }) => {
                reply.error(libc::ENOTDIR);
                return;
            }
        };

        // find the actual directory being deleted
        let inum = match files.get(filename) {
            Some(DirEntry {
                kind: FileType::Directory,
                inum,
            }) => inum,
            Some(_) => {
                reply.error(libc::ENOTDIR);
                return;
            }
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // make sure it's empty
        match self.get(*inum) {
            Ok(Inode {
                entry: Entry::Directory(_, dir_files),
                ..
            }) => {
                if !dir_files.is_empty() {
                    reply.error(libc::ENOTEMPTY);
                    return;
                }
            }
            Ok(_) => unreachable!("mismatched metadata on inode {} in parent {}", inum, parent),
            _ => unreachable!("couldn't find inode {} in parent {}", inum, parent),
        };

        // find the parent again, mutably
        let files = match self.get_mut(parent) {
            Ok(Inode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(_) => unreachable!("parent changed to a regular file"),
            Err(_) => unreachable!("error finding parent again"),
        };

        // try to remove it
        let res = files.remove(filename);
        assert!(res.is_some());
        self.dirty.set(true);
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn rename(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32, // TODO 2021-06-14 support RENAME_ flags
        reply: ReplyEmpty,
    ) {
        info!("called");

        // access control
        if !self.check_access(req) {
            reply.error(libc::EACCES);
            return;
        }

        let src = match name.to_str() {
            None => {
                reply.error(libc::ENOENT);
                return;
            }
            Some(name) => name,
        };

        if src == "." || src == ".." {
            reply.error(libc::EINVAL);
            return;
        }

        let tgt = match newname.to_str() {
            None => {
                reply.error(libc::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure src exists
        let (src_kind, src_inum) = match self.get(parent) {
            Ok(Inode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => match files.get(src) {
                Some(DirEntry { kind, inum }) => (*kind, *inum),
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            },
            _ => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        // determine whether tgt exists
        let tgt_info = match self.get(newparent) {
            Ok(Inode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => match files.get(tgt) {
                Some(DirEntry { kind, inum }) => {
                    if src_kind != *kind {
                        reply.error(libc::ENOTDIR);
                        return;
                    }
                    Some((*kind, *inum))
                }
                None => None,
            },
            _ => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // if tgt exists and is a directory, make sure it's empty
        if let Some((FileType::Directory, tgt_inum)) = tgt_info {
            match self.get(tgt_inum) {
                Ok(Inode {
                    entry: Entry::Directory(_type, files),
                    ..
                }) => {
                    if !files.is_empty() {
                        reply.error(libc::ENOTEMPTY);
                        return;
                    }
                }
                _ => unreachable!("bad metadata on inode {} in {}", tgt_inum, newparent),
            }
        }
        // remove src from parent
        match self.get_mut(parent) {
            Ok(Inode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => files.remove(src),
            _ => unreachable!("parent changed"),
        };

        // add src as tgt to newparent
        match self.get_mut(newparent) {
            Ok(Inode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => files.insert(
                tgt.into(),
                DirEntry {
                    kind: src_kind,
                    inum: src_inum,
                },
            ),
            _ => unreachable!("parent changed"),
        };

        // set src's parent inode
        match self.get_mut(src_inum) {
            Ok(inode) => inode.parent = newparent,
            Err(_) => unreachable!(
                "missing inode {} moved from {} to {}",
                src_inum, parent, newparent
            ),
        }

        self.dirty.set(true);
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn fallocate(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        if offset < 0 || length <= 0 {
            reply.error(libc::EINVAL);
            return;
        }

        if mode != 0 {
            reply.error(libc::EOPNOTSUPP);
            return;
        }

        // access control
        if !self.check_access(req) {
            reply.error(libc::EACCES);
            return;
        }

        // load the contents
        let contents = match self.get_mut(ino) {
            Ok(Inode {
                entry: Entry::File(contents),
                ..
            }) => contents,
            Ok(Inode {
                entry: Entry::Directory(..),
                ..
            }) => {
                reply.error(libc::EBADF);
                return;
            }
            Err(_e) => {
                reply.error(libc::ENODEV);
                return;
            }
        };

        // extend the vector
        let extra_bytes = (offset + length as i64) - contents.len() as i64;
        if extra_bytes > 0 {
            contents.resize(contents.len() + extra_bytes as usize, 0);
        }

        self.dirty.set(true);
        reply.ok()
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn fsync(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");
        reply.error(libc::ENOSYS);
    }

    // TODO
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn copy_file_range(
        &mut self,
        _req: &Request<'_>,
        _ino_in: u64,
        _fh_in: u64,
        _offset_in: i64,
        _ino_out: u64,
        _fh_out: u64,
        _offset_out: i64,
        _len: u64,
        _flags: u32,
        reply: ReplyWrite,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    // TODO
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn ioctl(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _cmd: u32,
        _in_data: &[u8],
        _out_size: u32,
        reply: ReplyIoctl,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    // Unimplemented/default-implementation calls
    #[instrument(level = "debug", skip(self, _req))]
    fn forget(&mut self, _req: &Request<'_>, _ino: u64, _nlookup: u64) {}

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readlink(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyData) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn symlink(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _link: &Path,
        reply: ReplyEntry,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn link(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        info!("called");

        // TODO 2021-06-16 access check?
        reply.opened(0, 0);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn flush(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.ok();
    }
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        info!("called");

        reply.opened(0, 0);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readdirplus(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.ok();
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn listxattr(&mut self, _req: &Request<'_>, _ino: u64, _size: u32, reply: ReplyXattr) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn removexattr(&mut self, _req: &Request<'_>, _ino: u64, _name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getlk(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        reply: ReplyLock,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn setlk(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        _sleep: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn bmap(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _blocksize: u32,
        _idx: u64,
        reply: ReplyBmap,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn lseek(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _whence: i32,
        reply: ReplyLseek,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[cfg(target_os = "macos")]
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn setvolname(&mut self, _req: &Request<'_>, _name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[cfg(target_os = "macos")]
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn exchange(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _newparent: u64,
        _newname: &OsStr,
        _options: u64,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(libc::ENOSYS);
    }

    #[cfg(target_os = "macos")]
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getxtimes(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyXTimes) {
        info!("called");

        reply.error(libc::ENOSYS);
    }
}
