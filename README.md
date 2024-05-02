# ffs: the file filesystem
[![Main workflow](https://github.com/mgree/ffs/actions/workflows/build.yml/badge.svg)](https://github.com/mgree/ffs/actions/workflows/build.yml)
[![Crates.io](https://img.shields.io/crates/v/ffs.svg)](https://crates.io/crates/ffs)

ffs, the **f**ile **f**ile**s**ystem, let's you mount semi-structured
data as a filesystem---a tree structure you already know how to work with!

Working with semi-structured data using command-line tools is hard.
Tools like [jq](https://github.com/stedolan/jq) help a lot, but
learning a new language for simple manipulations is a big ask. By mapping
hard-to-parse trees into a filesystem, you can keep using the tools you
know.

# Example

Run `ffs [file.blah]` to mount `file.blah` at the mountpoint `file`. The
final, updated version of the file will be outputted on stdout.

```shell-session
$ cat object.json 
{ "name": "Michael Greenberg", "eyes": 2, "fingernails": 10, "human": true }
$ ffs -o object_edited.json object.json &
[1] 60182
$ tree object
object
├── eyes
├── fingernails
├── human
└── name

0 directories, 4 files
$ echo Mikey Indiana >object/name
$ echo 1 >object/nose
$ mkdir object/pockets
$ cd object/pockets/
$ echo keys >pants
$ echo pen >shirt
$ cd ..
$ cd ..
$ umount object
$ 
[1]+  Done                    ffs -o object_edited.json object.json
$ cat object_edited.json 
{"eyes":2,"fingernails":10,"human":true,"name":"Mikey Indiana","nose":1,"pockets":{"pants":"keys","shirt":"pen"}}
```

You can specify an explicit mountpoint by running `ffs -m MOUNT file`;
you can specify an output file with `-o OUTPUT`. You can edit a file
in place by running `ffs -i file`---when the volume is unmounted, the
resulting output will be written back to `file`.

You can control whether directories are rendered as objects or arrays
lists using extended file attributes (xattrs): the `user.type` xattr
specifies `named` for objects and `list` for arrays. Here, we create a
new JSON file and use Linux's `setfattr` to mark a directory as being
a list (macOS alternatives are in comments):

```ShellSession
~$ ffs --new l.json &
[1] 287077
~$ cd l
~/l $ echo 'hi' >a
~/l $ echo 'bye' >b
~/l $ echo 'hello' >a1
~/l $ ls
a  a1  b
~/l $ cd ..
~$ setfattr -n user.type -v list l   # macOS: xattr -w user.type list l
~$ umount l
[1]+  Done                    ffs --new l.json
~$ cat l.json
["hi","hello","bye"]
```

# External dependencies

You need an appropriate [FUSE](https://github.com/libfuse/libfuse) or
[macFUSE](https://osxfuse.github.io/) along with
[pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/).

See [the GitHub build
workflow](https://github.com/mgree/ffs/blob/main/.github/workflows/build.yml)
for examples of external dependency installation.
