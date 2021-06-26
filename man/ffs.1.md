% FFS(1) Version 0.1.0 | File Filesystem Documentation
% Michael Greenberg

# NAME

ffs - the file filesystem

# SYNOPSIS

| ffs \[*FLAGS*\] \[*OPTIONS*\] \[*INPUT*\]
| ffs *--completions* *SHELL*
| ffs \[*-h*|*--help*\]
| ffs \[*-V*|*--version*\]


# DESCRIPTION

*ffs*---the *f*ile *f*ile*s*ystem---lets you mount semi-structured
data as a filesystem, allowing you to work with modern formats using
familiar shell tools.

*ffs* uses filesystems in userspace (FUSE); you must have these
installed on your system to use *ffs*.

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

```
curl https://api.github.com/repos/mgree/ffs/commits | ffs -m mnt -o commits.json 
```

If you had already downloaded the file to *commits.json*, you could simply run:

```
ffs -i commits.json
# do edits in commits directory
umount commits
# changes are written back to commits.json (-i is in-place mode)
```

To mount a JSON file and write back out a YAML file, you could run:

```
ffs -o output_data.yaml input_data.json
# do edits in the input_data directory ffs created
umount input_data
```

When filenames are present, extensions will be used to infer the
format being used. You can specify the source and target formats
explicitly with *--source* and *--target*, respectively.

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
