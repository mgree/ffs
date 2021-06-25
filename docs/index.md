---
title: "ffs: the file fileystem"
description: "mount semi-structured data (like JSON) as a Unix filesystem"
author: "[Michael Greenberg](http://mgree.github.io)"
---

The Unix shell is a powerful tool, and the Unix ecosystem provides an
incredible array of tools for working with strings. But the shell
really only knows how to work with one data *structure*: the
filesystem. Modern systems use all kinds of
[*semi-structured*](https://en.m.wikipedia.org/wiki/Semi-structured_data)
data, like JSON or YAML. These semi-structured formats are essentially
trees, and string processing is a bad match---editing JSON with sed is
not a very good idea!

ffs---short for the **f**ile **f**ile**s**ystem---lets you mount
semi-structured data as a filesystem, letting you work with modern
formats using your familiar shell tools.

Currently, ffs supports [JSON](https://www.json.org/),
[YAML](https://yaml.org/), and [TOML](https://toml.io/en/), with more
to come.

# Examples

Run `ffs [file]` to mount `file.blah` at the mountpoint `file`. The
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

Notice a few things: the `nose` key parsed out the number; the
`pockets` directory got turned into an object.

You can specify an explicit mountpoint by running `ffs -m MOUNT file`;
you can specify an output file with `-o OUTPUT`. You can edit a file
in place by running `ffs -i file`---when the volume is unmounted, the
resulting output will be written back to `file`.

# Getting ffs

See the [release page](https://github.com/mgree/ffs/releases).

You can also build ffs from [source](https://github.com/mgree/ffs). On
Linux you need [FUSE](https://github.com/libfuse/libfuse); on macOS,
you need [macFUSE](https://osxfuse.github.io/).

# Related tools

Tools like [jq](https://stedolan.github.io/jq/) and
[gron](https://github.com/tomnomnom/gron) are meant to help you work
with JSON on the command line. They're great tools!

Why might ffs be the right choice for you?

  - ffs supports multiple formats.

  - ffs lets you edit using familiar shell tools.
  
  - ffs doesn't involve learning a new language.

Why might ffs *not* be the right choice for you?

  - You use Windows. (Sorry. 😥)
  
  - You can't use FUSE.
  
  - You only need to search, not edit.
  
  - Your files are very large.

# License

ffs is licensed under
[GPLv3](https://raw.githubusercontent.com/mgree/ffs/main/LICENSE).
