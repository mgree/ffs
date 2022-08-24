binary data gets treated as base64

# unpack

JSON, TOML, YAML file -> file system hierarchy

```
# puts the foo data into the bar directory (making bar if it doesn't exist)
cat foo.json | unpack --into bar

# puts the foo data into the foo directory (making foo if it doesn't exists)
unpack foo.json

# in both of those, it's an error if foo or bar exist and are non-empty

# unpack stdin (coming from baz) into quux, treating input as YAML
cat baz | unpack -s yaml --into quux
```

src/format.rs describes mappings from these formats into the `Nodelike` trait

## a possible cut through the work:

- [ ] get JSON to work by hand

      write some tests

- [ ] get other formats work using `Nodelike`

  + wrinkle: YAML has a special notion of anchor that would be cool to treat as a sym- or hardlink
    problem not actually worth thinking about
  
      write some more tests
     
- [ ] implement options

  --debug
  --exact
  --no-xattr
  --quiet
  --time
  --unpadded
  --munge
  --dirmode, --mode, --gid, --uid
  -s, --source  # rename to -t, --type ?
  -m, --mount   # rename to -i, --into ?

  write tests of unpack
  write separate tests that compare ffs and unpack's behavior
    `diff -r` might do the trick
      xattr/uid/gid/mtime/etc. stuff is a bit more subtle

## things to think about

- [ ] read semi-structured data
  - default to stdin
  - but take a file (many files?!)
  
  output is... at a default mountpoint, or at a directory based on the filename
    follow ffs lead here

- [ ] options that matter

- [ ] build the directory tree, write the data, set some xattrs as necessary, that's it

- [ ] test

  follow the general lead of run_tests.sh and tests/*.sh
  
  how do we ensure that we don't hose the system?
  
    in docker?
    in `chroot`?
    with `pivot_root`?

# pack

file system hierarchy -> JSON, TOML, YAML file

```
# save /etc into a JSON file
pack /etc >config.json

pack -o lib.yaml /usr/share/lib

# -t specifying target type
pack -t toml . >bar.toml
```

- [ ] get it to work for just JSON

  + wrinkle: special file types (devices, FIFOs, etc.)
    what does tar do? gunzip unzip and one other to see what's standard
    
  + wrinkle: permissions
    `pack -o everything.json /`
    what does tar etc. do?
    
  + wrinkle: hard and symlinks
  
    hardlinks are just files... worst case we copy
      would be cool in YAML to have them be anchors
      
    symlinks can cause loops, can go outside of the root specified, etc.
      cf. `cp`, `tar`, `find` options `cp -L` to specify following symlinks, `--nofollow`
      but also: don't infinite loop
      PATH_MAX
      
      i think there are good rust libraries for filesystem traversal

- [ ] get it to work for `Nodelike` 

- [ ] implement options that matter

  --debug
  --keep-macos-xattr
  --pretty
  --time
  --munge
  --exact
  --quite
  --target
  --output

# testing wrt ffs

ffs and pack/unpack should behave as identically as possible
  we should explicitly test this on fixed and maybe also random inputs

# fuzzing

generate random inputs and run unpack on them

generate random filesystems and run pack on them (or run pack on random points in the FS)

fuzz ffs itself?

# performance

- [ ] think about ramdisks

- [ ] compare pack/unpack and ffs in a bunch of ways lol
