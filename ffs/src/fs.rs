use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use fuser::{Errno, INodeNo};
#[cfg(target_os = "linux")]
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyDirectoryPlus, ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen,
    ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};

use tracing::{debug, error, info, instrument, warn};

use nodelike::config::{Config, ERROR_STATUS_FUSE, Munge, Output};
use nodelike::time_ns;
use nodelike::{Format, Node, Nodelike, Typ, json, toml, yaml};

/// A filesystem `FS` is just a vector of nullable inodes, where the index is
/// the inode number.
///
/// NB that inode 0 is always invalid.
#[derive(Debug)]
pub struct FS<V: Nodelike> {
    pub state: Mutex<FSState<V>>,
}

#[derive(Debug)]
pub struct FSState<V: Nodelike> {
    /// Vector of nullable inodes; the index is the inode number.
    inodes: Vec<Option<INode<V>>>,
    /// Configuration, which determines various file attributes.
    config: Config,
    /// Dirty bit: set to `true` when there are outstanding writes
    dirty: bool,
    /// Synced bit: set to `true` if syncing has _ever_ happened
    synced: bool,
}

/// Default TTL on information passed to the OS, which caches responses.
const TTL: Duration = Duration::from_secs(300);

/// An inode, the core structure in the filesystem.
#[derive(Debug)]
pub struct INode<V: Nodelike> {
    /// Inode number of the parent of the current inode.
    ///
    /// For the root, it will be `FUSE_ROOT_ID`, i.e., itself.
    pub parent: INodeNo,
    /// Inode number of this node. Will not be 0.
    pub inum: INodeNo,
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
pub enum Entry<V: Nodelike> {
    // TODO 2021-06-14 need a 'written' flag to determine whether or not to
    // strip newlines during writeback
    File(Typ, Vec<u8>),
    Directory(DirType, BTreeMap<String, DirEntry>),
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
    pub inum: INodeNo,
}

#[derive(Debug)]
pub enum DirType {
    Named,
    List,
}

#[derive(Debug)]
#[allow(dead_code)] // better to have it saved for debugging!
enum FSError {
    NoSuchInode(INodeNo),
    InvalidInode(INodeNo),
}

impl<V: Nodelike> FS<V> {
    pub fn new(config: Config) -> Self {
        info!("loading");

        let reader = match config.input_reader() {
            Some(reader) => reader,
            None => {
                // create an empty directory
                let state = Mutex::new(FSState::empty(config));
                return Self { state };
            }
        };

        let v = time_ns!("reading", V::from_reader(reader), config.timing);
        if !v.is_dir() {
            error!(
                "The root of the filesystem must be a directory, but '{v}' only generates a single file."
            );
            std::process::exit(ERROR_STATUS_FUSE);
        }

        // don't bother with any locks until we've kicked things off
        let mut state = FSState::rooted(v, config);
        time_ns!(
            "loading",
            {
                if state.config.eager {
                    state
                        .resolve_nodes_transitively(fuser::INodeNo::ROOT)
                        .expect("resolve_nodes_transitively");
                } else {
                    // kick start the root directory
                    state
                        .resolve_node(fuser::INodeNo::ROOT)
                        .expect("resolve_node");
                }
            },
            state.config.timing
        );

        let state = Mutex::new(state);
        Self { state }
    }
}

impl<V: Nodelike> FSState<V> {
    fn from_root(root: INode<V>, config: Config) -> Self {
        let mut inodes: Vec<Option<INode<V>>> = Vec::with_capacity(1024);

        inodes.push(None);
        inodes.push(Some(root));

        let dirty = false;
        let synced = false;
        Self {
            inodes,
            config,
            dirty,
            synced,
        }
    }

    pub fn rooted(v: V, config: Config) -> Self {
        Self::from_root(
            INode::new(
                fuser::INodeNo::ROOT,
                fuser::INodeNo::ROOT,
                Entry::Lazy(v),
                &config,
            ),
            config,
        )
    }

    pub fn empty(config: Config) -> Self {
        Self::from_root(
            INode::new(
                fuser::INodeNo::ROOT,
                fuser::INodeNo::ROOT,
                Entry::Directory(DirType::Named, BTreeMap::new()),
                &config,
            ),
            config,
        )
    }

    fn fresh_inode(
        &mut self,
        parent: INodeNo,
        entry: Entry<V>,
        uid: u32,
        gid: u32,
        mode: u32,
    ) -> INodeNo {
        self.dirty = true;

        let inum = INodeNo(self.inodes.len() as u64);
        let mode = (mode & 0o777) as u16;

        self.inodes
            .push(Some(INode::with_mode(parent, inum, entry, uid, gid, mode)));

        inum
    }

    #[instrument(level = "debug", skip(self))]
    fn resolve_node(&mut self, inum: INodeNo) -> Result<Option<Vec<INodeNo>>, FSError> {
        debug!("called");

        let idx = inum.0 as usize;

        if idx >= self.inodes.len() || idx == 0 {
            return Err(FSError::NoSuchInode(inum));
        }

        {
            let inode = match self.inodes[idx].as_ref() {
                Some(inode) => inode,
                None => return Err(FSError::InvalidInode(inum)),
            };
            match inode.entry {
                Entry::Directory(..) | Entry::File(..) => return Ok(Option::None),
                Entry::Lazy(..) => {}
            }
        }

        // Take ownership of the inode so we can move the lazy value out without
        // needing Default. The slot is temporarily None while we build children.
        let mut inode = self.inodes[idx].take().unwrap();
        let uid = inode.uid;
        let gid = inode.gid;
        let v = match inode.entry {
            Entry::Lazy(v) => v,
            _ => return Err(FSError::InvalidInode(inum)),
        };

        let (entry, new_nodes) = match v.node(&self.config) {
            Node::Bytes(b) => (Entry::File(Typ::Bytes, b), Option::None),
            Node::String(t, s) => (Entry::File(t, s.into_bytes()), Option::None),
            Node::List(vs) => {
                let mut children = BTreeMap::new();
                let num_elts = vs.len() as f64;
                let width = num_elts.log10().ceil() as usize;

                let mut new_nodes = Vec::with_capacity(vs.len());
                for (i, child) in vs.into_iter().enumerate() {
                    // TODO 2021-06-08 ability to add prefixes
                    let name = if self.config.pad_element_names {
                        format!("{i:0width$}")
                    } else {
                        format!("{i}")
                    };

                    let kind = filetype_for(&child);
                    let child_id = self.fresh_inode(
                        inum,
                        Entry::Lazy(child),
                        uid,
                        gid,
                        mode(&self.config, kind) as u32,
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
                let mut children = BTreeMap::new();
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
                                warn!("skipping '{field}'");
                                continue;
                            }
                        }
                    } else {
                        field
                    };

                    let kind = filetype_for(&child);
                    let child_id = self.fresh_inode(
                        inum,
                        Entry::Lazy(child),
                        uid,
                        gid,
                        mode(&self.config, kind) as u32,
                    );
                    let original_name = if original != nfield {
                        info!(
                            "renamed {original} to {nfield} (inode {child_id} with parent {inum})"
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

        inode.entry = entry;
        self.inodes[idx] = Some(inode);

        if let Some(nodes) = &new_nodes {
            debug!("new_nodes = {nodes:?}");
        }

        Ok(new_nodes)
    }

    fn resolve_nodes_transitively(&mut self, inum: INodeNo) -> Result<(), FSError> {
        let mut worklist = match self.resolve_node(inum)? {
            Some(nodes) => nodes,
            None => return Ok(()),
        };

        while let Some(node) = worklist.pop() {
            if let Some(nodes) = self.resolve_node(node)? {
                worklist.extend(nodes);
            }
        }

        Ok(())
    }

    fn get(&mut self, inum: INodeNo) -> Result<&INode<V>, FSError> {
        let _new_nodes = self.resolve_node(inum)?;

        let idx = inum.0 as usize;

        if idx >= self.inodes.len() || idx == 0 {
            return Err(FSError::NoSuchInode(inum));
        }

        match &self.inodes[idx] {
            None => Err(FSError::InvalidInode(inum)),
            Some(inode) => Ok(inode),
        }
    }

    fn get_mut(&mut self, inum: INodeNo) -> Result<&mut INode<V>, FSError> {
        let _new_nodes = self.resolve_node(inum)?;

        let idx = inum.0 as usize;

        if idx >= self.inodes.len() {
            return Err(FSError::NoSuchInode(inum));
        }

        match self.inodes.get_mut(idx) {
            Some(Some(inode)) => Ok(inode),
            _ => Err(FSError::InvalidInode(inum)),
        }
    }

    #[instrument(level = "trace", skip(self))]
    fn as_other_value<U>(&mut self, inum: INodeNo) -> U
    where
        U: Nodelike,
    {
        match &self.inodes[inum.0 as usize].as_ref().unwrap().entry {
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
                        warn!("skipping ignored file '{name}'");
                        continue;
                    }
                    let v = self.as_other_value(inum);
                    entries.push(v);
                }
                U::from_list_dir(entries, &self.config)
            }
            Entry::Directory(DirType::Named, files) => {
                let mut entries = BTreeMap::new();

                let files = files
                    .iter()
                    .map(|(name, entry)| (name.clone(), entry.inum, entry.original_name.clone()))
                    .collect::<Vec<_>>();
                for (name, inum, original_name) in files.iter() {
                    if self.config.ignored_file(name) {
                        warn!("skipping ignored file '{name}'");
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

    fn write_as_format(&mut self, format: Format, writer: Box<dyn std::io::Write>, pretty: bool) {
        match format {
            Format::Json => time_ns!(
                "writing",
                self.as_other_value::<json::Value>(fuser::INodeNo::ROOT)
                    .to_writer(writer, pretty),
                self.config.timing
            ),
            Format::Toml => time_ns!(
                "writing",
                self.as_other_value::<toml::Value>(fuser::INodeNo::ROOT)
                    .to_writer(writer, pretty),
                self.config.timing
            ),
            Format::Yaml => time_ns!(
                "writing",
                self.as_other_value::<yaml::Value>(fuser::INodeNo::ROOT)
                    .to_writer(writer, pretty),
                self.config.timing
            ),
        }
    }

    fn check_access(&self, req: &Request) -> bool {
        req.uid() == 0 || req.uid() == self.config.uid
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
    #[instrument(level = "debug", skip(self), fields(synced = self.synced, dirty = self.dirty))]
    pub fn sync(&mut self, last_sync: bool) {
        info!("called");

        if self.synced && !self.dirty {
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
        self.dirty = false;
        self.synced = true;
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

        let output_format = self.config.output_format;
        let pretty = self.config.pretty;
        time_ns!(
            "saving",
            self.write_as_format(output_format, writer, pretty),
            self.config.timing
        );
    }
}

impl<V: Nodelike> INode<V> {
    pub fn new(parent: INodeNo, inum: INodeNo, entry: Entry<V>, config: &Config) -> Self {
        let mode = mode(config, entry.kind());
        let uid = config.uid;
        let gid = config.gid;
        INode::with_mode(parent, inum, entry, uid, gid, mode)
    }

    pub fn with_mode(
        parent: INodeNo,
        inum: INodeNo,
        entry: Entry<V>,
        uid: u32,
        gid: u32,
        mode: u16,
    ) -> Self {
        let now = SystemTime::now();

        INode {
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
            Entry::Lazy(..) => panic!("unresolved lazy value in Inode::attr"),
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

impl<V: Nodelike> Entry<V> {
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
                files.keys().map(|name| name.len() as u64).sum()
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
            Entry::Lazy(v) => filetype_for(v),
        }
    }

    pub fn typ(&self) -> String {
        match self {
            Entry::File(t, _) => t.to_string(),
            Entry::Directory(t, _) => t.to_string(),
            Entry::Lazy(_) => panic!("unresolved lazy value in Entry::typ"),
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

fn filetype_for(node: &dyn Nodelike) -> FileType {
    if node.is_dir() {
        FileType::Directory
    } else {
        FileType::RegularFile
    }
}

/// Determines the default mode of a file
fn mode(config: &Config, kind: FileType) -> u16 {
    if kind == FileType::Directory {
        config.dirmode
    } else {
        config.filemode
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
const ENOATTR: fuser::Errno = Errno::ENODATA;

impl<V: Nodelike + 'static> Filesystem for FS<V> {
    /// Synchronizes the `FS`, calling `FS::sync` with `last_sync == true`.
    #[instrument(level = "debug", skip(self))]
    fn destroy(&mut self) {
        info!("called");
        self.state.lock().unwrap().sync(true);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn statfs(&self, _req: &Request, _inode: INodeNo, reply: ReplyStatfs) {
        info!("called");
        reply.statfs(0, 0, 0, 0, 0, 1, 255, 0);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn access(&self, req: &Request, inum: INodeNo, mask: fuser::AccessFlags, reply: ReplyEmpty) {
        info!("called");
        if mask == fuser::AccessFlags::F_OK {
            reply.ok();
            return;
        }

        let mut state = self.state.lock().unwrap();
        match state.get(inum) {
            Ok(inode) => {
                // cribbed from https://github.com/cberner/fuser/blob/4639a490f4aa7dfe8a342069a761d4cf2bd8f821/examples/simple.rs#L1703-L1736
                let attr = inode.attr();
                let mode = attr.perm as i32;

                // TODO actually use the bitflags
                let mut mask = mask.bits();
                if req.uid() == 0 {
                    // root only allowed to exec if one of the X bits is set
                    mask &= libc::X_OK;
                    mask -= mask & (mode >> 6);
                    mask -= mask & (mode >> 3);
                    mask -= mask & mode;
                } else if req.uid() == state.config.uid {
                    mask -= mask & (mode >> 6);
                } else if req.gid() == state.config.gid {
                    mask -= mask & (mode >> 3);
                } else {
                    mask -= mask & mode;
                }

                if mask == 0 {
                    reply.ok();
                } else {
                    reply.error(Errno::EACCES);
                }
            }
            Err(_) => reply.error(Errno::ENOENT),
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        let dir = match state.get(parent) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        let filename = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        let inum = match &dir.entry {
            Entry::Directory(_kind, files) => match files.get(filename) {
                None => {
                    reply.error(Errno::ENOENT);
                    return;
                }
                Some(DirEntry { inum, .. }) => *inum,
            },
            _ => {
                reply.error(Errno::ENOTDIR);
                return;
            }
        };

        let file = match state.get(inum) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        reply.entry(&TTL, &file.attr(), fuser::Generation(0));
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: ReplyAttr,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        let file = match state.get(ino) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
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
        &self,
        req: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<fuser::FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<fuser::BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        if !state.check_access(req) {
            reply.error(Errno::EPERM);
            return;
        }

        if let Some(mode) = mode {
            info!("chmod to {mode:o}");

            if mode != mode & 0o777 {
                info!("truncating mode {mode:o} to {:o}", mode & 0o777);
            }
            let mode = (mode as u16) & 0o777;

            match state.get_mut(ino) {
                Ok(inode) => {
                    inode.mode = mode;
                    reply.attr(&TTL, &inode.attr());
                    return;
                }
                Err(_) => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            };
        }

        // cribbing from https://github.com/cberner/fuser/blob/13557921548930afd6b70e109521044fea98c23b/examples/simple.rs#L594-L639
        if uid.is_some() || gid.is_some() {
            info!("chown called with uid {uid:?} gid {gid:?}");

            // gotta be a member of the target group!
            if let Some(gid) = gid {
                let groups = groups_for(req.uid());
                if req.uid() != 0 && !groups.contains(&gid) {
                    reply.error(Errno::EPERM);
                    return;
                }
            }

            let inode = match state.get_mut(ino) {
                Ok(inode) => inode,
                Err(_) => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            };

            // non-root owner can only do noop uid changes
            if let Some(uid) = uid
                && req.uid() != 0
                && !(uid == inode.uid && req.uid() == inode.uid)
            {
                reply.error(Errno::EPERM);
                return;
            }

            // only owner may change the group
            if gid.is_some() && req.uid() != 0 && req.uid() != inode.uid {
                reply.error(Errno::EPERM);
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
            info!("truncate() to {size}");

            match state.get_mut(ino) {
                Ok(inode) => match &mut inode.entry {
                    Entry::File(_t, contents) => {
                        contents.resize(size as usize, 0);
                        reply.attr(&TTL, &inode.attr());
                    }
                    Entry::Directory(..) => {
                        reply.error(Errno::EISDIR);
                        return;
                    }
                    Entry::Lazy(..) => panic!("unresolved lazy value found in setattr"),
                },
                Err(_) => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            };

            state.dirty = true;
            return;
        }

        let now = SystemTime::now();
        let mut set_time = false;

        if let Some(atime) = atime {
            info!("setting atime");
            if !state.check_access(req) {
                reply.error(Errno::EPERM);
                return;
            }

            match state.get_mut(ino) {
                Ok(inode) => {
                    inode.atime = match atime {
                        TimeOrNow::Now => now,
                        TimeOrNow::SpecificTime(time) => time,
                    }
                }
                Err(_) => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            }

            set_time = true;
        }

        if let Some(mtime) = mtime {
            info!("setting mtime");

            if !state.check_access(req) {
                reply.error(Errno::EPERM);
                return;
            }
            match state.get_mut(ino) {
                Ok(inode) => {
                    inode.mtime = match mtime {
                        TimeOrNow::Now => now,
                        TimeOrNow::SpecificTime(time) => time,
                    }
                }
                Err(_) => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            }

            set_time = true;
        }

        if set_time {
            reply.attr(&TTL, &state.get(ino).unwrap().attr());
        } else {
            reply.error(Errno::ENOSYS);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getxattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, size: u32, reply: ReplyXattr) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        if !state.config.allow_xattr {
            info!("disabled");
            reply.error(Errno::ENOSYS);
            return;
        }

        let file = match state.get(ino) {
            Err(_e) => {
                reply.error(Errno::EFAULT);
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
                reply.error(Errno::ERANGE);
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
        &self,
        req: &Request,
        ino: INodeNo,
        name: &OsStr,
        value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        if !state.config.allow_xattr {
            reply.error(Errno::ENOSYS);
            return;
        }

        if !state.check_access(req) {
            reply.error(Errno::EPERM);
            return;
        }

        let file = match state.get_mut(ino) {
            Err(_e) => {
                reply.error(Errno::EFAULT);
                return;
            }
            Ok(inode) => inode,
        };

        if name == "user.type" {
            match std::str::from_utf8(value) {
                Err(_) => {
                    reply.error(Errno::EINVAL);
                }
                Ok(s) => {
                    if file.entry.try_set_typ(s) {
                        reply.ok()
                    } else {
                        reply.error(Errno::EINVAL)
                    }
                }
            }
        } else {
            reply.error(Errno::EINVAL);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn listxattr(&self, _req: &Request, ino: INodeNo, size: u32, reply: ReplyXattr) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        if !state.config.allow_xattr {
            reply.error(Errno::ENOSYS);
            return;
        }

        if state.get(ino).is_err() {
            reply.error(Errno::EFAULT);
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
            reply.error(Errno::ERANGE);
        } else {
            reply.data(&attrs);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn removexattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        // 50 ways to leave your lover: this call never succeeds

        let mut state = self.state.lock().unwrap();
        if !state.config.allow_xattr {
            reply.error(Errno::ENOSYS);
            return;
        }

        if state.get(ino).is_err() {
            reply.error(Errno::EFAULT);
            return;
        }

        if name == "user.type" {
            reply.error(Errno::EACCES);
        } else {
            reply.error(ENOATTR);
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        _size: u32,
        _flags: fuser::OpenFlags,
        _lock: Option<fuser::LockOwner>,
        reply: ReplyData,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        let file = match state.get(ino) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        match &file.entry {
            Entry::File(_t, s) => reply.data(&s[offset as usize..]),
            _ => reply.error(Errno::ENOENT),
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();
        let inode = match state.get(ino) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        match &inode.entry {
            Entry::File(..) => reply.error(Errno::ENOTDIR),
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
                    .enumerate()
                    .skip(offset as usize)
                {
                    if reply.add(entry.0, (i + 1) as u64, entry.1, entry.2) {
                        break;
                    }
                }
                reply.ok()
            }
            Entry::Lazy(..) => panic!("unresolved lazy value in readdir"),
        }
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn create(
        &self,
        _req: &Request,
        _parent: INodeNo,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        info!("called");

        // force the system to use mknod and open
        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn mknod(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // make sure we have a good file type
        let file_type: u32 = mode & libc::S_IFMT;
        if ![libc::S_IFREG, libc::S_IFDIR].contains(&file_type) {
            warn!("mknod only supports regular files and directories; got {mode:o}");
            reply.error(Errno::ENOSYS);
            return;
        }

        // get the filename
        let filename = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure the parent exists, is a directory, and doesn't have that file
        match state.get(parent) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => match &inode.entry {
                Entry::File(..) => {
                    reply.error(Errno::ENOTDIR);
                    return;
                }
                Entry::Directory(_dirtype, files) => {
                    if files.contains_key(filename) {
                        reply.error(Errno::EEXIST);
                        return;
                    }
                }
                Entry::Lazy(..) => panic!("unresolved lazy value in mknod"),
            },
        };

        // create the inode entry
        let (entry, kind) = if file_type == libc::S_IFREG {
            (Entry::File(Typ::Auto, Vec::new()), FileType::RegularFile)
        } else {
            assert_eq!(file_type, libc::S_IFDIR);
            (
                Entry::Directory(DirType::Named, BTreeMap::new()),
                FileType::Directory,
            )
        };

        // allocate the inode (sets dirty bit)
        let inum = state.fresh_inode(parent, entry, req.uid(), req.gid(), mode);

        // update the parent
        // NB we can't get_mut the parent earlier due to borrowing restrictions
        match state.get_mut(parent) {
            Err(_e) => panic!("error finding parent again"),
            Ok(inode) => match &mut inode.entry {
                Entry::File(..) => panic!("parent changed to a regular file"),
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
                Entry::Lazy(..) => panic!("unresolved lazy value in mknod"),
            },
        };

        reply.entry(
            &TTL,
            &state.get(inum).unwrap().attr(),
            fuser::Generation(0),
        );
        assert!(state.dirty);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn mkdir(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // get the new directory name
        let filename = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure the parent exists, is a directory, and doesn't have anything with that name
        match state.get(parent) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => match &inode.entry {
                Entry::File(..) => {
                    reply.error(Errno::ENOTDIR);
                    return;
                }
                Entry::Directory(_dirtype, files) => {
                    if files.contains_key(filename) {
                        reply.error(Errno::EEXIST);
                        return;
                    }
                }
                Entry::Lazy(..) => panic!("unresolved lazy value in mkdir"),
            },
        };

        // create the inode entry
        let entry = Entry::Directory(DirType::Named, BTreeMap::new());
        let kind = FileType::Directory;

        // allocate the inode (sets dirty bit)
        let inum = state.fresh_inode(parent, entry, req.uid(), req.gid(), mode);

        // update the parent
        // NB we can't get_mut the parent earlier due to borrowing restrictions
        match state.get_mut(parent) {
            Err(_e) => panic!("error finding parent again"),
            Ok(inode) => match &mut inode.entry {
                Entry::File(..) => panic!("parent changed to a regular file"),
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
                Entry::Lazy(..) => panic!("unresolved lazy value in mkdir"),
            },
        };

        reply.entry(
            &TTL,
            &state.get(inum).unwrap().attr(),
            fuser::Generation(0),
        );
        assert!(state.dirty);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn write(
        &self,
        req: &Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: fuser::WriteFlags,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyWrite,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // find inode
        let file = match state.get_mut(ino) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(inode) => inode,
        };

        // load contents
        let contents = match &mut file.entry {
            Entry::File(_t, contents) => contents,
            Entry::Directory(_, _) => {
                reply.error(Errno::EISDIR);
                return;
            }
            Entry::Lazy(..) => panic!("unresolved lazy value in write"),
        };

        // make space
        let extra_bytes = (offset + data.len() as u64) - contents.len() as u64;
        if extra_bytes > 0 {
            contents.resize(contents.len() + extra_bytes as usize, 0);
        }

        // actually write
        let offset = offset as usize;
        contents[offset..offset + data.len()].copy_from_slice(data);
        state.dirty = true;

        reply.written(data.len() as u32);
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn unlink(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // get the filename
        let filename = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // find the parent
        let files = match state.get_mut(parent) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(INode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(INode {
                entry: Entry::File(..),
                ..
            }) => {
                reply.error(Errno::ENOTDIR);
                return;
            }
            Ok(INode {
                entry: Entry::Lazy(..),
                ..
            }) => panic!("unresolved lazy value in unlink"),
        };

        // ensure it's a regular file
        match files.get(filename) {
            Some(DirEntry {
                kind: FileType::RegularFile,
                ..
            }) => (),
            _ => {
                reply.error(Errno::EPERM);
                return;
            }
        }

        // try to remove it
        let res = files.remove(filename);
        assert!(res.is_some());
        state.dirty = true;
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn rmdir(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // get the filename
        let filename = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // find the parent
        let files = match state.get(parent) {
            Err(_e) => {
                reply.error(Errno::ENOENT);
                return;
            }
            Ok(INode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(INode {
                entry: Entry::File(..),
                ..
            }) => {
                reply.error(Errno::ENOTDIR);
                return;
            }
            Ok(INode {
                entry: Entry::Lazy(..),
                ..
            }) => panic!("unresolved lazy value in rmdir"),
        };

        // find the actual directory being deleted
        let inum = match files.get(filename) {
            Some(DirEntry {
                kind: FileType::Directory,
                inum,
                ..
            }) => *inum,
            Some(_) => {
                reply.error(Errno::ENOTDIR);
                return;
            }
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        // make sure it's empty
        match state.get(inum) {
            Ok(INode {
                entry: Entry::Directory(_, dir_files),
                ..
            }) => {
                if !dir_files.is_empty() {
                    reply.error(Errno::ENOTEMPTY);
                    return;
                }
            }
            Ok(_) => panic!("mismatched metadata on inode {inum} in parent {parent}"),
            _ => panic!("couldn't find inode {inum} in parent {parent}"),
        };

        // find the parent again, mutably
        let files = match state.get_mut(parent) {
            Ok(INode {
                entry: Entry::Directory(_dirtype, files),
                ..
            }) => files,
            Ok(_) => panic!("parent changed to a regular file"),
            Err(_) => panic!("error finding parent again"),
        };

        // try to remove it
        let res = files.remove(filename);
        assert!(res.is_some());
        state.dirty = true;
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn rename(
        &self,
        req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        _flags: fuser::RenameFlags, // TODO 2021-06-14 support RENAME_ flags
        reply: ReplyEmpty,
    ) {
        info!("called");

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        let src = match name.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        if src == "." || src == ".." {
            reply.error(Errno::EINVAL);
            return;
        }

        let tgt = match newname.to_str() {
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
            Some(name) => name,
        };

        // make sure src exists
        let (src_kind, src_original, src_inum) = match state.get(parent) {
            Ok(INode {
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
                    reply.error(Errno::ENOENT);
                    return;
                }
            },
            _ => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        // determine whether tgt exists
        let tgt_info = match state.get(newparent) {
            Ok(INode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => match files.get(tgt) {
                Some(DirEntry { kind, inum, .. }) => {
                    if src_kind != *kind {
                        reply.error(Errno::ENOTDIR);
                        return;
                    }
                    Some((*kind, *inum))
                }
                None => None,
            },
            _ => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        // if tgt exists and is a directory, make sure it's empty
        if let Some((FileType::Directory, tgt_inum)) = tgt_info {
            match state.get(tgt_inum) {
                Ok(INode {
                    entry: Entry::Directory(_type, files),
                    ..
                }) => {
                    if !files.is_empty() {
                        reply.error(Errno::ENOTEMPTY);
                        return;
                    }
                }
                _ => panic!("bad metadata on inode {tgt_inum} in {newparent}"),
            }
        }
        // remove src from parent
        match state.get_mut(parent) {
            Ok(INode {
                entry: Entry::Directory(_kind, files),
                ..
            }) => files.remove(src),
            _ => panic!("parent changed"),
        };

        // add src as tgt to newparent
        match state.get_mut(newparent) {
            Ok(INode {
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
            _ => panic!("parent changed"),
        };

        // set src's parent inode
        match state.get_mut(src_inum) {
            Ok(inode) => inode.parent = newparent,
            Err(_) => panic!("missing inode {src_inum} moved from {parent} to {newparent}"),
        }

        state.dirty = true;
        reply.ok();
    }

    #[instrument(level = "debug", skip(self, req, reply))]
    fn fallocate(
        &self,
        req: &Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        length: u64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        info!("called");

        if mode != 0 {
            reply.error(Errno::EOPNOTSUPP);
            return;
        }

        let mut state = self.state.lock().unwrap();

        // access control
        if !state.check_access(req) {
            reply.error(Errno::EACCES);
            return;
        }

        // load the contents
        let contents = match state.get_mut(ino) {
            Ok(INode {
                entry: Entry::File(_t, contents),
                ..
            }) => contents,
            Ok(INode {
                entry: Entry::Directory(..),
                ..
            }) => {
                reply.error(Errno::EBADF);
                return;
            }
            Ok(INode {
                entry: Entry::Lazy(..),
                ..
            }) => panic!("unresolved lazy value in fallocate"),

            Err(_e) => {
                reply.error(Errno::ENODEV);
                return;
            }
        };

        // extend the vector
        let extra_bytes = (offset + length) - (contents.len() as u64);
        if extra_bytes > 0 {
            contents.resize(contents.len() + extra_bytes as usize, 0);
        }

        state.dirty = true;
        reply.ok()
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn fsync(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");
        reply.error(Errno::ENOSYS);
    }

    // TODO
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn copy_file_range(
        &self,
        _req: &Request,
        _ino_in: fuser::INodeNo,
        _fh_in: fuser::FileHandle,
        _offset_in: u64,
        _ino_out: fuser::INodeNo,
        _fh_out: fuser::FileHandle,
        _offset_out: u64,
        _len: u64,
        _flags: fuser::CopyFileRangeFlags,
        reply: ReplyWrite,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    // TODO
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn ioctl(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _flags: fuser::IoctlFlags,
        _cmd: u32,
        _in_data: &[u8],
        _out_size: u32,
        reply: ReplyIoctl,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    // Unimplemented/default-implementation calls
    #[instrument(level = "debug", skip(self, _req))]
    fn forget(&self, _req: &Request, _ino: INodeNo, _nlookup: u64) {}

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readlink(&self, _req: &Request, _ino: INodeNo, reply: ReplyData) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn symlink(
        &self,
        _req: &Request,
        _parent: INodeNo,
        _name: &OsStr,
        _link: &Path,
        reply: ReplyEntry,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn link(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _newparent: INodeNo,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn open(&self, _req: &Request, _ino: INodeNo, _flags: fuser::OpenFlags, reply: ReplyOpen) {
        info!("called");

        // TODO 2021-06-16 access check?
        reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn flush(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _lock_owner: fuser::LockOwner,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _flags: fuser::OpenFlags,
        _lock_owner: std::option::Option<fuser::LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.ok();
    }
    #[instrument(level = "debug", skip(self, _req, reply))]
    fn opendir(&self, _req: &Request, _ino: INodeNo, _flags: fuser::OpenFlags, reply: ReplyOpen) {
        info!("called");

        reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn readdirplus(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _offset: u64,
        reply: ReplyDirectoryPlus,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn releasedir(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _flags: fuser::OpenFlags,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.ok();
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn fsyncdir(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn getlk(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _lock_owner: fuser::LockOwner,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        reply: ReplyLock,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn setlk(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _lock_owner: fuser::LockOwner,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        _sleep: bool,
        reply: ReplyEmpty,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn bmap(&self, _req: &Request, _ino: INodeNo, _blocksize: u32, _idx: u64, reply: ReplyBmap) {
        info!("called");

        reply.error(Errno::ENOSYS);
    }

    #[instrument(level = "debug", skip(self, _req, reply))]
    fn lseek(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _offset: i64,
        _whence: i32,
        reply: ReplyLseek,
    ) {
        info!("called");

        reply.error(Errno::ENOSYS);
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
