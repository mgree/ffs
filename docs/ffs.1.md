% FFS(1) Version 0.1.0 | File Filesystem Documentation
% Michael Greenberg

# NAME

ffs - the file filesystem

# SYNOPSIS

| ffs \[*FLAGS*\] \[*OPTIONS*\] \[*INPUT*\]
| ffs *--completions* *SHELL*
| ffs \[*-h*\|*--help*\]
| ffs \[*-V*\|*--version*\]


# DESCRIPTION

*ffs*---the *f*ile *f*ile*s*ystem---lets you mount semi-structured
data as a filesystem, allowing you to work with modern formats using
familiar shell tools.

*ffs* uses filesystems in userspace (FUSE); you must have these
installed on your system to use *ffs*. 

*ffs* expects its input to be encoded in UTF-8.

## Flags

-d, --debug

: Give debug output on stderr

--exact

: Don't add newlines to the end of values that don't already have them
  (or strip them when loading)

-i, --in-place

: Writes the output back over the input file

--no-output

: Disables output of filesystem (normally on stdout)

-q, --quiet

: Quiet mode (turns off all errors and warnings, enables
  *--no-output*)

--readonly

: Mounted filesystem will be readonly

--unpadded

: Don't pad the numeric names of list elements with zeroes; will not
  sort properly

--no-xattr

: Don't use extended attributes to track metadata (see `man xattr`)

--keep-macos-xattr

: Include ._* extended attribute/resource fork files on macOS.

-h, --help

: Prints help information (and exits)

-V, --version

: Prints version information (and exits)

## Options

--dirmode *DIRMODE*

: Sets the default mode of directories (parsed as octal; if
  unspecified, directories will have *FILEMODE*, with execute bits set
  when read bits are set) [default: 755]

--mode *FILEMODE*

: Sets the default mode of files (parsed as octal) [default: 644]

-g, --gid *GID*

: Sets the group id of the generated filesystem (defaults to current
  effective group id)

-m, --mount *MOUNT*

: Sets the mountpoint; will be inferred when using a file, but must be
  specified when running on stdin

-o, --output *OUTPUT*

: Sets the output file for saving changes (defaults to stdout)

--completions *SHELL*

: Generate shell completions and exit [possible values: bash, fish,
  zsh]

-s, --source *SOURCE_FORMAT*

: Specify the source format explicitly (by default, automatically
  inferred from filename extension) [possible values: json, toml,
  yaml]

-t, --target *TARGET_FORMAT*

: Specify the target format explicitly (by default, automatically
  inferred from filename extension) [possible values: json, toml,
  yaml]

-u, --uid *UID*

: Sets the user id of the generated filesystem (defaults to current
  effective user id)

## Arguments
 
*INPUT*

: Sets the input file (use '-' for stdin) [default: -]

## Data model

The data model for *ffs* is a superset of that of its supported
formats (currently, JSON, TOML, and YAML); *ffs* maps values in these
formats to filesystems. Here are the different types and how they're
mapped to a filesystem:

Boolean

: Mapped to a **file**. Either *true* or *false*.

Bytes

: Mapped to a **file**. When serializing back to format, it will be encoded in base64.

Datetime

: Mapped to a **file**. Some portion of an [RFC
  3339](https://datatracker.ietf.org/doc/html/rfc3339) date/time.

Integer

: Mapped to a **file**. No larger than 64 bits.

Float

: Mapped to a **file**. No larger than 64 bits.

List

: Mapped to a **directory**. List directories will have numerically
  named elements, starting from 0. Filenames will be padded with zeros
  to ensure proper sorting; use *--unpadded* to disable padding. While
  mounted, you are free to use whatever filenames you like in a list
  directory. When list directories are serialized back to a format,
  filenames are ignored and the sorted order of the files (in the
  current locale) will be used to determine the list order.

Named

: Mapped to a **directory**. Named directories (also known as maps,
  objects, hashes, or dictionaries) will use field names as the
  file/directory names for their contents. Some renaming may occur if
  fields have special characters in them.

Null

: Mapped to a **file**. The file will be empty.

String

: Mapped to a **file**. The file will be encoded in UTF-8 holding the
  string.

By default every file will have a newline appended to its contents;
this newline will be removed when the filesystem is dumped back to a
file. To disable these newlines, use *--exact*.

You can inspect and alter the types of files and directories using
extended attributes (use `xattr` on macOS and
`attr`/`getfattr`/`setfattr` on Linux).

# ENVIRONMENT

RUST_LOG

: Configures tracing output. Use the format *key*=*level*, where *key*
  should probably be *ffs* and *level* should be one of *error*,
  *warn*, *info*, *debug*, or *trace*. The default is
  *ffs=warn*. Setting *-q* turns off all output; setting *-d* sets
  *ffs=debug*.

# EXAMPLES

The general workflow is to run *ffs*, do some work, and then unmount
the mountpoint using *umount*. It's typical to run *ffs* in the
background, since the program will not terminate until the userspace
filesystem is unmounted.

By default, *ffs* will work off of stdin, in which case you must
specify a mountpoint with *-m*.  If you have a mountpoint/directory
*mnt*, you can download information on GitHub commits, work with them,
and save the modified output to *commits.json* by running:

```shell
curl https://api.github.com/repos/mgree/ffs/commits | ffs -m mnt -o commits.json 
```

If you had already downloaded the file to *commits.json*, you could simply run:

```shell
ffs -i commits.json
# do edits in commits directory
umount commits
# changes are written back to commits.json (-i is in-place mode)
```

To mount a JSON file and write back out a YAML file, you could run:

```shell
ffs -o output_data.yaml input_data.json
# do edits in the input_data directory ffs created
umount input_data
```

When filenames are present, extensions will be used to infer the
format being used. You can specify the source and target formats
explicitly with *--source* and *--target*, respectively.

You can use extended attributes to change a list directory to a named
one (or vice versa); this example uses macOS, with Linux alternatives
in comments.

```ShellSession
$ ffs -i list.json &
[1] 41361
$ cat list.json
[1,2,"3",false]
$ cd list
$ mv 0 loneliest_number
$ mv 1 to_tango
$ mv 2 three
$ mv 3 not_true
$ xattr -l .                    # Linux: getattr --match=- .
user.type: list
$ xattr -w user.type named .    # Linux: setattr -n user.type -v named .
$ ls
loneliest_number not_true         three            to_tango
$ cd ..
$ umount list
$
[1]+  Done                    target/debug/ffs -i list.json
$ cat list.json
{"loneliest_number":1,"not_true":false,"three":"3","to_tango":2}
```

# SEE ALSO

fuse(4), fusermount(3), mount(8), umount(8)

To learn more about FUSE (Filesystem in Userspace), check out libfuse
(Linux)
[https://github.com/libfuse/libfuse](https://github.com/libfuse/libfuse)
and macFUSE (macOS)
[https://osxfuse.github.io/](https://osxfuse.github.io/).

# BUGS

See
[https://github.com/mgree/ffs/issues](https://github.com/mgree/ffs/issues).

# LICENSE

Copyright 2021 (c) Michael Greenberg. GPL-3.0 licensed.
