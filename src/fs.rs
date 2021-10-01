use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{Debug, Display};
use std::mem;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

#[cfg(target_os = "macos")]
use fuser::ReplyXTimes;

use tracing::{debug, error, info, instrument, trace, warn};

use super::config::{Config, Munge, Output, ERROR_STATUS_FUSE};
use super::format::{json, toml, yaml, Format, Node, Nodelike, Typ};
use crate::time_ns;

/// A filesystem `FS` is just a vector of nullable inodes, where the index is
/// the inode number.
///
/// NB that inode 0 is always invalid.
#[derive(Debug)]
pub struct FS<V>
where
    V: Nodelike + Clone + Debug + std::fmt::Display,
{
    /// Vector of nullable inodes; the index is the inode number.
    pub inodes: Vec<Option<Inode<V>>>,
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
pub struct Inode<V> {
    /// Inode number of the parent of the current inode.
    ///
    /// For the root, it will be `FUSE_ROOT_ID`, i.e., itself.
    pub parent: u64,
    /// Inode number of this node. Will not be 0.
    pub inum: u64,
    /// User ID of the owner
    pub uid: u32,
    /// Group ID of the owner,
    pub gid: u32,
    /// Mode of this inode. Defaults to values set in `FS.config`, but calls to
    /// `mknod` and `mkdir` and `setattr` (as `chmod`) can change this.
    pub mode: u16,
    /// Time of last access
    pub atime: SystemTime,
    /// Time of last modification
    pub mtime: SystemTime,
    /// Time of last change
    pub ctime: SystemTime,
    /// Time of creation (macOS only)
    pub crtime: SystemTime,
    /// The actual file contents.
    pub entry: Entry<V>,
}

/// File contents. Either a `File` containing bytes or a `Directory`, mapping
/// names to entries (see `DirEntry`)
///
/// Directories come in two kinds (per `DirType`): `DirType::Named` directories
/// are conventional mappings of names to entries, but `DirType::List`
/// directories only use name in the filesystem, and most of those names will be
/// generated (see `format::fs_from_value`). When writing a `DirType::List`
/// directory back out, only the sort order of the name matters.
#[derive(Debug)]
pub enum Entry<V> {
    // TODO 2021-06-14 need a 'written' flag to determine whether or not to
    // strip newlines during writeback
    File(Typ, Vec<u8>),
    Directory(DirType, HashMap<String, DirEntry>),
    Lazy(V),
}

/// Directory entries. We record the kind and inode (for faster
/// `Filesystem::readdir`).
#[derive(Clone, Debug)]
pub struct DirEntry {
    pub kind: FileType,
    /// When loading from certain map types, names might get munged.
    /// We store the original name here so we can restore it appropriately.
    ///
    /// If the file is renamed, we'll drop the original name.
    pub original_name: Option<String>,
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

impl<V> FS<V>
where
    V: Nodelike + Clone + Debug + Display + Default,
{
    fn fresh_inode(&mut self, parent: u64, entry: Entry<V>, uid: u32, gid: u32, mode: u32) -> u64 {
        self.dirty.set(true);

        let inum = self.inodes.len() as u64;
        let mode = (mode & 0o777) as u16;

        self.inodes
            .push(Some(Inode::with_mode(parent, inum, entry, uid, gid, mode)));

        inum
    }

    #[instrument(level = "debug", skip(self))]
    fn resolve_node(&mut self, inum: u64) -> Result<Option<Vec<u64>>, FSError>
    where
        V: Nodelike + std::fmt::Display + Default,
    {
        debug!("called");

        let idx = inum as usize;

        if idx >= self.inodes.len() || idx == 0 {
            return Err(FSError::NoSuchInode(inum));
        }

        let inode = match &mut self.inodes[idx] {
            Some(inode) => inode,
            _ => return Err(FSError::InvalidInode(inum)),
        };

        let v = match &mut inode.entry {
            Entry::Directory(..) | Entry::File(..) => return Ok(Option::None),
            Entry::Lazy(v) => mem::take(v),
        };
        let uid = inode.uid;
        let gid = inode.gid;

        let (entry, new_nodes) = match v.node(&self.config) {
            Node::Bytes(b) => (Entry::File(Typ::Bytes, b), Option::None),
            Node::String(t, s) => (Entry::File(t, s.into_bytes()), Option::None),
            Node::List(vs) => {
                let mut children = HashMap::new();
                children.reserve(vs.len());
                let num_elts = vs.len() as f64;
                let width = num_elts.log10().ceil() as usize;

                let mut new_nodes = Vec::with_capacity(vs.len());
                for (i, child) in vs.into_iter().enumerate() {
                    // TODO 2021-06-08 ability to add prefixes
                    let name = if self.config.pad_element_names {
                        format!("{:0width$}", i, width = width)
                    } else {
                        format!("{}", i)
                    };

                    let kind = child.kind();
                    let child_id = self.fresh_inode(
                        inum,
                        Entry::Lazy(child),
                        uid,
                        gid,
                        self.config.mode(kind) as u32,
                    );

                    children.insert(
                        name,
                        DirEntry {
                            kind,
                            original_name: None,
                            inum: child_id,
                        },
                    );
                    new_nodes.push(child_id)
                }

                (
                    Entry::Directory(DirType::List, children),
                    Option::Some(new_nodes),
                )
            }
            Node::Map(fvs) => {
                let mut children = HashMap::new();
                children.reserve(fvs.len());

                let mut new_nodes = Vec::with_capacity(fvs.len());
                for (field, child) in fvs.into_iter() {
                    let original = field.clone();

                    let nfield = if !self.config.valid_name(&original) {
                        match self.config.munge {
                            Munge::Rename => {
                                let mut nfield = self.config.normalize_name(field);

                                // TODO 2021-07-08 could be better to check fvs, but it's a vec now... :/
                                while children.contains_key(&nfield) {
                                    nfield.push('_');
                                }

                                nfield
                            }
                            Munge::Filter => {
                                warn!("skipping '{}'", field);
                                continue;
                            }
                        }
                    } else {
                        field
                    };

                    let kind = child.kind();
                    let child_id = self.fresh_inode(
                        inum,
                        Entry::Lazy(child),
                        uid,
                        gid,
                        self.config.mode(kind) as u32,
                    );
                    let original_name = if original != nfield {
                        info!(
                            "renamed {} to {} (inode {} with parent {})",
                            original, nfield, child_id, inum
                        );
                        Some(original)
                    } else {
                        assert!(self.config.valid_name(&original));
                        None
                    };

                    children.insert(
                        nfield,
                        DirEntry {
                            kind,
                            original_name,
                            inum: child_id,
                        },
                    );

                    new_nodes.push(child_id);
                }

                (
                    Entry::Directory(DirType::Named, children),
                    Option::Some(new_nodes),
                )
            }
        };

        let inode = match &mut self.inodes[idx] {
            Some(inode) => inode,
            _ => return Err(FSError::InvalidInode(inum)),
        };
        inode.entry = entry;

        if let Some(nodes) = &new_nodes {
            debug!("new_nodes = {:?}", nodes);
        }

        Ok(new_nodes)
    }

    fn resolve_nodes_transitively(&mut self, inum: u64) -> Result<(), FSError> {
        let mut worklist = match self.resolve_node(inum)? {
            Some(nodes) => nodes,
            None => return Ok(()),
        };

        while !worklist.is_empty() {
            let node = worklist.pop().unwrap();
            if let Some(nodes) = self.resolve_node(node)? {
                worklist.extend(nodes);
            }
        }

        Ok(())
    }

    fn check_access(&self, req: &Request) -> bool {
        req.uid() == 0 || req.uid() == self.config.uid
    }

    pub fn get(&mut self, inum: u64) -> Result<&Inode<V>, FSError> {
        let _new_nodes = self.resolve_node(inum)?;

        let idx = inum as usize;

        if idx >= self.inodes.len() || idx == 0 {
            return Err(FSError::NoSuchInode(inum));
        }

        match &self.inodes[idx] {
            None => Err(FSError::InvalidInode(inum)),
            Some(inode) => Ok(inode),
        }
    }

    fn get_mut(&mut self, inum: u64) -> Result<&mut Inode<V>, FSError> {
        let _new_nodes = self.resolve_node(inum)?;

        let idx = inum as usize;

        if idx >= self.inodes.len() {
            return Err(FSError::NoSuchInode(inum));
        }

        match self.inodes.get_mut(idx) {
            Some(Some(inode)) => Ok(inode),
            _ => Err(FSError::InvalidInode(inum)),
        }
    }

    pub fn new(config: Config) -> Self {
        info!("loading");
        let mut inodes: Vec<Option<Inode<V>>> = Vec::with_capacity(1024);
        // allocate space for dummy inode 0, root node
        inodes.resize_with(2, || None);

        let reader = match config.input_reader() {
            Some(reader) => reader,
            None => {
                // create an empty directory
                let contents = HashMap::with_capacity(16);
                inodes[1] = Some(Inode::new(
                    fuser::FUSE_ROOT_ID,
                    fuser::FUSE_ROOT_ID,
                    Entry::Directory(DirType::Named, contents),
                    &config,
                ));
                return FS {
                    inodes,
                    config,
                    dirty: Cell::new(false),
                    synced: Cell::new(false),
                };
            }
        };

        let v = time_ns!("reading", V::from_reader(reader), config.timing);
        if v.kind() != FileType::Directory {
            error!("The root of the filesystem must be a directory, but '{}' only generates a single file.", v);
            std::process::exit(ERROR_STATUS_FUSE);
        }

        let mut fs = FS {
            inodes,
            config,
            dirty: Cell::new(false),
            synced: Cell::new(false),
        };

        time_ns!(
            "loading",
            {
                fs.inodes[fuser::FUSE_ROOT_ID as usize] = Option::Some(Inode::new(
                    fuser::FUSE_ROOT_ID,
                    fuser::FUSE_ROOT_ID,
                    Entry::Lazy(v),
                    &fs.config,
                ));

                if fs.config.eager {
                    fs.resolve_nodes_transitively(fuser::FUSE_ROOT_ID)
                        .expect("resolve_nodes_transitively");
                } else {
                    // kick start the root directory
                    fs.resolve_node(fuser::FUSE_ROOT_ID).expect("resolve_node");
                }
            },
            fs.config.timing
        );

        fs
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
    #[instrument(level = "debug", skip(self), fields(synced = self.synced.get(), dirty = self.dirty.get()))]
    pub fn sync(&mut self, last_sync: bool) {
        info!("called");
        trace!("{:?}", self.inodes);

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

        self.save();
        self.dirty.set(false);
        self.synced.set(true);
    }

    /// Actually output results, using `self.config.output`.
    ///
    /// When `self.config.input == self.config.output`, then resolved lazy nodes
    /// can be directly returned. If the input and output formats are different,
    /// we eager resolve everything and then save.
    fn save(&mut self) {
        let writer = match self.config.output_writer() {
            Some(writer) => writer,
            None => return,
        };

        if self.config.input_format == self.config.output_format {
            let v = time_ns!(
                "saving",
                self.as_value(fuser::FUSE_ROOT_ID),
                self.config.timing
            );

            time_ns!(
                "writing",
                v.to_writer(writer, self.config.pretty),
                self.config.timing
            );
        } else {
            match self.config.output_format {
                Format::Json => {
                    let v: json::Value = time_ns!(
                        "saving",
                        self.as_other_value(fuser::FUSE_ROOT_ID),
                        self.config.timing
                    );

                    time_ns!(
                        "writing",
                        v.to_writer(writer, self.config.pretty),
                        self.config.timing
                    );
                }
                Format::Toml => {
                    let v: toml::Value = time_ns!(
                        "saving",
                        self.as_other_value(fuser::FUSE_ROOT_ID),
                        self.config.timing
                    );

                    time_ns!(
                        "writing",
                        v.to_writer(writer, self.config.pretty),
                        self.config.timing
                    );
                }
                Format::Yaml => {
                    let v: yaml::Value = time_ns!(
                        "saving",
                        self.as_other_value(fuser::FUSE_ROOT_ID),
                        self.config.timing
                    );

                    time_ns!(
                        "writing",
                        v.to_writer(writer, self.config.pretty),
                        self.config.timing
                    );
                }
            }
        }
    }

    // save as a value of the same type as the input
    // we need this special case to avoid type-level shenanigans
    fn as_value(&self, inum: u64) -> V {
        match &self.inodes[inum as usize].as_ref().unwrap().entry {
            Entry::Lazy(v) => v.clone(),
            Entry::File(typ, contents) => {
                // TODO 2021-07-01 use _t to try to force the type
                match String::from_utf8(contents.clone()) {
                    Ok(mut contents) if typ != &Typ::Bytes => {
                        if self.config.add_newlines && contents.ends_with('\n') {
                            contents.truncate(contents.len() - 1);
                        }
                        // TODO 2021-06-24 trim?
                        V::from_string(*typ, contents, &self.config)
                    }
                    Ok(_) | Err(_) => V::from_bytes(contents, &self.config),
                }
            }
            Entry::Directory(DirType::List, files) => {
                let mut entries = Vec::with_capacity(files.len());
                let mut files = files.iter().collect::<Vec<_>>();
                files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
                for (name, DirEntry { inum, .. }) in files.iter() {
                    if self.config.ignored_file(name) {
                        warn!("skipping ignored file '{}'", name);
                        continue;
                    }
                    let v = self.as_value(*inum);
                    entries.push(v);
                }
                V::from_list_dir(entries, &self.config)
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
                    if self.config.ignored_file(name) {
                        warn!("skipping ignored file '{}'", name);
                        continue;
                    }
                    let v = self.as_value(*inum);
                    let name = original_name.as_ref().unwrap_or(name).into();
                    entries.insert(name, v);
                }
                V::from_named_dir(entries, &self.config)
            }
        }
    }

    #[instrument(level = "trace", skip(self))]
    fn as_other_value<U>(&mut self, inum: u64) -> U
    where
        U: Nodelike,
    {
        match &self.inodes[inum as usize].as_ref().unwrap().entry {
            Entry::Lazy(_) => {
                self.resolve_nodes_transitively(inum).unwrap();
                self.as_other_value(inum)
            }
            Entry::File(typ, contents) => {
                // TODO 2021-07-01 use _t to try to force the type
                match String::from_utf8(contents.clone()) {
                    Ok(mut contents) if typ != &Typ::Bytes => {
                        if self.config.add_newlines && contents.ends_with('\n') {
                            contents.truncate(contents.len() - 1);
                        }
                        // TODO 2021-06-24 trim?
                        U::from_string(*typ, contents, &self.config)
                    }
                    Ok(_) | Err(_) => U::from_bytes(contents, &self.config),
                }
            }
            Entry::Directory(DirType::List, files) => {
                let mut entries = Vec::with_capacity(files.len());
                let mut files = files
                    .iter()
                    .map(|(name, entry)| (name.clone(), entry.inum))
                    .collect::<Vec<_>>();
                files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
                for (name, inum) in files {
                    if self.config.ignored_file(&name) {
                        warn!("skipping ignored file '{}'", name);
                        continue;
                    }
                    let v = self.as_other_value(inum);
                    entries.push(v);
                }
                U::from_list_dir(entries, &self.config)
            }
            Entry::Directory(DirType::Named, files) => {
                let mut entries = HashMap::with_capacity(files.len());

                let files = files
                    .iter()
                    .map(|(name, entry)| (name.clone(), entry.inum, entry.original_name.clone()))
                    .collect::<Vec<_>>();
                for (name, inum, original_name) in files.iter() {
                    if self.config.ignored_file(name) {
                        warn!("skipping ignored file '{}'", name);
                        continue;
                    }
                    let v = self.as_other_value(*inum);
                    let name = original_name.as_ref().unwrap_or(name).into();
                    entries.insert(name, v);
                }
                U::from_named_dir(entries, &self.config)
            }
        }
    }
}

impl<V> Inode<V>
where
    V: Nodelike,
{
    pub fn new(parent: u64, inum: u64, entry: Entry<V>, config: &Config) -> Self {
        let mode = config.mode(entry.kind());
        let uid = config.uid;
        let gid = config.gid;
        Inode::with_mode(parent, inum, entry, uid, gid, mode)
    }

    pub fn with_mode(
        parent: u64,
        inum: u64,
        entry: Entry<V>,
        uid: u32,
        gid: u32,
        mode: u16,
    ) -> Self {
        let now = SystemTime::now();

        Inode {
            parent,
            inum,
            uid,
            gid,
            mode,
            entry,
            atime: now,
            crtime: now,
            ctime: now,
            mtime: now,
        }
    }

    /// Gets the `FileAttr` of a given `Inode`. Some of this is computed each
    /// time: the size, the kind, permissions, and number of hard links.
    pub fn attr(&self) -> FileAttr {
        let size = self.entry.size();
        let kind = self.entry.kind();

        let nlink: u32 = match &self.entry {
            Entry::Directory(_, files) => {
                2 + files
                    .iter()
                    .filter(|(_, de)| de.kind == FileType::Directory)
                    .count() as u32
            }
            Entry::File(..) => 1,
            Entry::Lazy(..) => unreachable!("unresolved lazy value in Inode::attr"),
        };

        FileAttr {
            ino: self.inum,
            atime: self.atime,
            crtime: self.crtime,
            ctime: self.ctime,
            mtime: self.mtime,
            nlink,
            size,
            blksize: 1,
            blocks: size,
            kind,
            uid: self.uid,
            gid: self.gid,
            perm: self.mode,
            rdev: 0,
            flags: 0, // weird macOS thing
        }
    }
}

impl<V> Entry<V>
where
    V: Nodelike,
{
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
            Entry::File(_t, s) => s.len() as u64,
            Entry::Directory(DirType::Named, files) => {
                files.iter().map(|(name, _inum)| name.len() as u64).sum()
            }
            Entry::Directory(DirType::List, files) => files.len() as u64,
            Entry::Lazy(v) => v.size() as u64, // give an answer because we can... but should
        }
    }

    /// Determines the `FileType` of an `Entry`
    pub fn kind(&self) -> FileType {
        match self {
            Entry::File(..) => FileType::RegularFile,
            Entry::Directory(..) => FileType::Directory,
            Entry::Lazy(v) => v.kind(),
        }
    }

    pub fn typ(&self) -> String {
        match self {
            Entry::File(t, _) => t.to_string(),
            Entry::Directory(t, _) => t.to_string(),
            Entry::Lazy(_) => unreachable!("unresolved lazy value in Entry::typ"),
        }
    }

    /// Tries to set the type from a given string, returning `false` on an
    /// error.
    pub fn try_set_typ(&mut self, s: &str) -> bool {
        match self {
            Entry::File(typ, _) => match str::parse(s) {
                Ok(new_typ) => {
                    *typ = new_typ;
                    true
                }
                Err(..) => false,
            },
            Entry::Directory(typ, _) => match str::parse(s) {
                Ok(new_typ) => {
                    *typ = new_typ;
                    true
                }
                Err(..) => false,
            },
            Entry::Lazy(_) => todo!("Entry::try_set_typ"),
        }
    }
}

impl std::fmt::Display for DirType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                DirType::List => "list",
                DirType::Named => "named",
            }
        )
    }
}

impl FromStr for DirType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let s = s.trim().to_lowercase();

        if s == "list" || s == "array" {
            Ok(DirType::List)
        } else if s == "named"
            || s == "object"
            || s == "map"
            || s == "hash"
            || s == "dict"
            || s == "dictionary"
        {
            Ok(DirType::Named)
        } else {
            Err(())
        }
    }
}

// ENOATTR is deprecated on Linux, so we should use ENODATA
#[cfg(target_os = "linux")]
const ENOATTR: i32 = libc::ENODATA;
#[cfg(target_os = "macos")]
const ENOATTR: i32 = libc::ENOATTR;

impl<V> Filesystem for FS<V>
where
    V: Nodelike,
{
    /// Synchronizes the `FS`, calling `FS::sync` with `last_sync == true`.
    #[instrument(level = "debug", skip(self), fields(dirty = self.dirty.get()))]
    fn destroy(&mut self) {
        info!("called");
        self.sync(true);
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
                let attr = inode.attr();
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

        let inum = match &dir.entry {
            Entry::Directory(_kind, files) => match files.get(filename) {
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
                Some(DirEntry { inum, .. }) => *inum,
            },
            _ => {
                reply.error(libc::ENOTDIR);
                return;
            }
        };

        let file = match self.get(inum) {
            Err(_e) => {
                reply.error(libc::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        reply.entry(&TTL, &file.attr(), 0);
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

        reply.attr(&TTL, &file.attr());
    }

    #[instrument(
        level = "debug",
        skip(
            self, req, reply, mode, uid, gid, size, atime, mtime, _ctime, _fh, _crtime, _chgtime,
            _bkuptime, _flags
        )
    )]
    fn setattr(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        info!("called");

        if !self.check_access(req) {
            reply.error(libc::EPERM);
            return;
        }

        if let Some(mode) = mode {
            info!("chmod to {:o}", mode);

            if mode != mode & 0o777 {
                info!("truncating mode {:o} to {:o}", mode, mode & 0o777);
            }
            let mode = (mode as u16) & 0o777;

            match self.get_mut(ino) {
                Ok(inode) => {
                    inode.mode = mode;
                    reply.attr(&TTL, &inode.attr());
                    return;
                }
                Err(_) => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
        }

        // cribbing from https://github.com/cberner/fuser/blob/13557921548930afd6b70e109521044fea98c23b/examples/simple.rs#L594-L639
        if uid.is_some() || gid.is_some() {
            info!("chown called with uid {:?} guid {:?}", uid, gid);

            // gotta be a member of the target group!
            if let Some(gid) = gid {
                let groups = groups_for(req.uid());
                if req.uid() != 0 && !groups.contains(&gid) {
                    reply.error(libc::EPERM);
                    return;
                }
            }

            let inode = match self.get_mut(ino) {
                Ok(inode) => inode,
                Err(_) => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };

            // non-root owner can only do noop uid changes
            if let Some(uid) = uid {
                if req.uid() != 0 && !(uid == inode.uid && req.uid() == inode.uid) {
                    reply.error(libc::EPERM);
                    return;
                }
            }

            // only owner may change the group
            if gid.is_some() && req.uid() != 0 && req.uid() != inode.uid {
                reply.error(libc::EPERM);
                return;
            }

            // NB if we allowed SETUID/SETGID bits, we might need to clear them here
            if let Some(uid) = uid {
                inode.uid = uid;
            }

            if let Some(gid) = gid {
                inode.gid = gid;
            }

            inode.ctime = SystemTime::now();
            reply.attr(&TTL, &inode.attr());
            return;
        }

        if let Some(size) = size {
            info!("truncate() to {}", size);

            match self.get_mut(ino) {
                Ok(inode) => match &mut inode.entry {
                    Entry::File(_t, contents) => {
                        contents.resize(size as usize, 0);
                        reply.attr(&TTL, &inode.attr());
                    }
                    Entry::Directory(..) => {
                        reply.error(libc::EISDIR);
                        return;
                    }
                    Entry::Lazy(..) => unreachable!("unresolved lazy value found in setattr"),
                },
                Err(_) => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };

            self.dirty.set(true);
            return;
        }

        let now = SystemTime::now();
        let mut set_time = false;
        if let Some(atime) = atime {
            info!("setting atime");
            if !self.check_access(req) {
                reply.error(libc::EPERM);
                return;
            }
            match self.get_mut(ino) {
                Ok(inode) => {
                    inode.atime = match atime {
                        TimeOrNow::Now => now,
                        TimeOrNow::SpecificTime(time) => time,
                    }
                }
                Err(_) => {
                    reply.error(libc::ENOENT);
                    return;
                }
            }

            set_time = true;
        }

        if let Some(mtime) = mtime {
            info!("setting mtime");

            if !self.check_access(req) {
                reply.error(libc::EPERM);
                return;
            }
            match self.get_mut(ino) {
                Ok(inode) => {
                    inode.mtime = match mtime {
                        TimeOrNow::Now => now,
                        TimeOrNow::SpecificTime(time) => time,
                    }
                }
                Err(_) => {
                    reply.error(libc::ENOENT);
                    return;
                }
            }

            set_time = true;
        }

        if set_time {
            reply.attr(&TTL, &self.get(ino).unwrap().attr());
        } else {
            reply.error(libc::ENOSYS);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        info!("called");

        if !self.config.allow_xattr {
            info!("disabled");
            reply.error(libc::ENOSYS);
            return;
        }

        let file = match self.get(ino) {
            Err(_e) => {
                reply.error(libc::EFAULT);
                return;
            }
            Ok(inode) => inode,
        };

        if name == "user.type" {
            let user_type = file.entry.typ().into_bytes();
            let actual_size = user_type.len() as u32;

            if size == 0 {
                reply.size(actual_size);
                return;
            } else if size < actual_size {
                reply.error(libc::ERANGE);
                return;
            } else {
                reply.data(&user_type);
                return;
            }
        }

        reply.error(ENOATTR);
    }

    #[instrument(level = "debug", skip(self, req, reply, value, _flags, _position))]
    fn setxattr(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        if !self.config.allow_xattr {
            reply.error(libc::ENOSYS);
            return;
        }

        if !self.check_access(req) {
            reply.error(libc::EPERM);
            return;
        }

        let file = match self.get_mut(ino) {
            Err(_e) => {
                reply.error(libc::EFAULT);
                return;
            }
            Ok(inode) => inode,
        };

        if name == "user.type" {
            match std::str::from_utf8(value) {
                Err(_) => {
                    reply.error(libc::EINVAL);
                }
                Ok(s) => {
                    if file.entry.try_set_typ(s) {
                        reply.ok()
                    } else {
                        reply.error(libc::EINVAL)
                    }
                }
            }
        } else {
            reply.error(libc::EINVAL);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn listxattr(&mut self, _req: &Request<'_>, ino: u64, size: u32, reply: ReplyXattr) {
        info!("called");

        if !self.config.allow_xattr {
            reply.error(libc::ENOSYS);
            return;
        }

        if self.get(ino).is_err() {
            reply.error(libc::EFAULT);
            return;
        }

        // TODO 2021-07-02
        // - we could add user.original_name here when present
        // - we could use a clearer name (e.g., `user.ffs.type`)
        let mut attrs: Vec<u8> = "user.type".into();
        attrs.push(0);
        let actual_size = attrs.len() as u32;

        if size == 0 {
            reply.size(actual_size)
        } else if size < actual_size {
            reply.error(libc::ERANGE);
        } else {
            reply.data(&attrs);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn removexattr(&mut self, _req: &Request<'_>, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        // 50 ways to leave your lover: this call never succeeds

        if !self.config.allow_xattr {
            reply.error(libc::ENOSYS);
            return;
        }

        if self.get(ino).is_err() {
            reply.error(libc::EFAULT);
            return;
        }

        if name == "user.type" {
            reply.error(libc::EACCES);
        } else {
            reply.error(ENOATTR);
        }
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
            Entry::File(_t, s) => reply.data(&s[offset as usize..]),
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
            Entry::File(..) => reply.error(libc::ENOTDIR),
            Entry::Directory(_kind, files) => {
                let dot_entries = vec![
                    (ino, FileType::Directory, "."),
                    (inode.parent, FileType::Directory, ".."),
                ];

                let entries = files.iter().map(|(filename, DirEntry { inum, kind, .. })| {
                    (*inum, *kind, filename.as_str())
                });

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
            Entry::Lazy(..) => unreachable!("unresolved lazy value in readdir"),
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
                Entry::File(..) => {
                    reply.error(libc::ENOTDIR);
                    return;
                }
                Entry::Directory(_dirtype, files) => {
                    if files.contains_key(filename) {
                        reply.error(libc::EEXIST);
                        return;
                    }
                }
                Entry::Lazy(..) => unreachable!("unresolved lazy value in mknod"),
            },
        };

        // create the inode entry
        let (entry, kind) = if file_type == libc::S_IFREG as u32 {
            (Entry::File(Typ::Auto, Vec::new()), FileType::RegularFile)
        } else {
            assert_eq!(file_type, libc::S_IFDIR as u32);
            (
                Entry::Directory(DirType::Named, HashMap::new()),
                FileType::Directory,
            )
        };

        // allocate the inode (sets dirty bit)
        let inum = self.fresh_inode(parent, entry, req.uid(), req.gid(), mode);

        // update the parent
        // NB we can't get_mut the parent earlier due to borrowing restrictions
        match self.get_mut(parent) {
            Err(_e) => unreachable!("error finding parent again"),
            Ok(inode) => match &mut inode.entry {
                Entry::File(..) => unreachable!("parent changed to a regular file"),
                Entry::Directory(_dirtype, files) => {
                    files.insert(
                        filename.into(),
                        DirEntry {
                            kind,
                            original_name: None,
                            inum,
                        },
                    );
                }
                Entry::Lazy(..) => unreachable!("unresolved lazy value in mknod"),
            },
        };

        reply.entry(&TTL, &self.get(inum).unwrap().attr(), 0);
        assert!(self.dirty.get());
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
                Entry::File(..) => {
                    reply.error(libc::ENOTDIR);
                    return;
                }
                Entry::Directory(_dirtype, files) => {
                    if files.contains_key(filename) {
                        reply.error(libc::EEXIST);
                        return;
                    }
                }
                Entry::Lazy(..) => unreachable!("unresolved lazy value in mkdir"),
            },
        };

        // create the inode entry
        let entry = Entry::Directory(DirType::Named, HashMap::new());
        let kind = FileType::Directory;

        // allocate the inode (sets dirty bit)
        let inum = self.fresh_inode(parent, entry, req.uid(), req.gid(), mode);

        // update the parent
        // NB we can't get_mut the parent earlier due to borrowing restrictions
        match self.get_mut(parent) {
            Err(_e) => unreachable!("error finding parent again"),
            Ok(inode) => match &mut inode.entry {
                Entry::File(..) => unreachable!("parent changed to a regular file"),
                Entry::Directory(_dirtype, files) => {
                    files.insert(
                        filename.into(),
                        DirEntry {
                            kind,
                            original_name: None,
                            inum,
                        },
                    );
                }
                Entry::Lazy(..) => unreachable!("unresolved lazy value in mkdir"),
            },
        };

        reply.entry(&TTL, &self.get(inum).unwrap().attr(), 0);
        assert!(self.dirty.get());
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
            Entry::File(_t, contents) => contents,
            Entry::Directory(_, _) => {
                reply.error(libc::EISDIR);
                return;
            }
            Entry::Lazy(..) => unreachable!("unresolved lazy value in write"),
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
                entry: Entry::File(..),
                ..
            }) => {
                reply.error(libc::ENOTDIR);
                return;
            }
            Ok(Inode {
                entry: Entry::Lazy(..),
                ..
            }) => unreachable!("unresolved lazy value in unlink"),
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
                entry: Entry::File(..),
                ..
            }) => {
                reply.error(libc::ENOTDIR);
                return;
            }
            Ok(Inode {
                entry: Entry::Lazy(..),
                ..
            }) => unreachable!("unresolved lazy value in rmdir"),
        };

        // find the actual directory being deleted
        let inum = match files.get(filename) {
            Some(DirEntry {
                kind: FileType::Directory,
                inum,
                ..
            }) => *inum,
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
        match self.get(inum) {
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
        let (src_kind, src_original, src_inum) = match self.get(parent) {
            Ok(Inode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => match files.get(src) {
                Some(DirEntry {
                    kind,
                    original_name,
                    inum,
                    ..
                }) => (*kind, original_name.clone(), *inum),
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
                Some(DirEntry { kind, inum, .. }) => {
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
                    // if the filename is the same, we'll keep the source
                    // original filename (if it exists; otherwise we overwrite
                    // it)
                    original_name: if src == tgt { src_original } else { None },
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
                entry: Entry::File(_t, contents),
                ..
            }) => contents,
            Ok(Inode {
                entry: Entry::Directory(..),
                ..
            }) => {
                reply.error(libc::EBADF);
                return;
            }
            Ok(Inode {
                entry: Entry::Lazy(..),
                ..
            }) => unreachable!("unresolved lazy value in fallocate"),

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

/// Returns the group IDs a user is in
#[cfg(target_os = "macos")]
fn groups_for(uid: u32) -> Vec<u32> {
    unsafe {
        let passwd = libc::getpwuid(uid);
        let name = (*passwd).pw_name;
        let basegid = (*passwd).pw_gid as i32;

        // get the number of groups
        let mut ngroups = 0;
        libc::getgrouplist(name, basegid, std::ptr::null_mut(), &mut ngroups);

        if ngroups == 0 {
            // BUG 2021-06-23 weird behavior on macos... :/
            ngroups = 50;
        }

        let mut groups = vec![-1; ngroups as usize];
        loop {
            libc::getgrouplist(name, basegid, groups.as_mut_ptr(), &mut ngroups);

            // if the last entry wasn't set, we're good
            if groups[groups.len() - 1] == -1 {
                break;
            }

            // otherwise, there are more groups. oof, keep going.
            ngroups *= 2;
            groups.resize(ngroups as usize, 0);
        }
        groups
            .into_iter()
            .filter(|gid| gid != &-1)
            .map(|gid| gid as u32)
            .collect()
    }
}

#[cfg(target_os = "linux")]
fn groups_for(uid: u32) -> Vec<u32> {
    unsafe {
        let passwd = libc::getpwuid(uid);
        let name = (*passwd).pw_name;
        let basegid = (*passwd).pw_gid;

        // get the number of groups
        let mut ngroups = 0;
        libc::getgrouplist(name, basegid, std::ptr::null_mut(), &mut ngroups);
        let mut groups = vec![0; ngroups as usize];
        let res = libc::getgrouplist(name, basegid, groups.as_mut_ptr(), &mut ngroups);
        assert_eq!(res, ngroups);
        groups
    }
}
