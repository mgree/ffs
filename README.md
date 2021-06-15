# ffs: the file filesystem
[![Main workflow](https://github.com/mgree/ffs/actions/workflows/build.yml/badge.svg)](https://github.com/mgree/ffs/actions/workflows/build.yml)

ffs, the **f**ile **f**ile**s**sytem, let's you mount semi-structured
data as a fileystem---a tree structure you already know how to work with!

Working with semi-structured data using command-line tools is hard.
Tools like [jq](https://github.com/stedolan/jq) help a lot, but
learning a new language for simple manipulations is a big ask. By mapping
hard-to-parse trees into a filesystem, you can keep using the tools you
know.

# Example

Run `ffs [mountpoint] [file]` to mount a file at a given mountpoint.

```shell-session
$ ffs mnt json_eg1.json  &
[1] 80762
$ tree mnt
mnt
└── glossary
    ├── GlossDiv
    │   ├── GlossList
    │   │   └── GlossEntry
    │   │       ├── Abbrev
    │   │       ├── Acronym
    │   │       ├── GlossDef
    │   │       │   ├── GlossSeeAlso
    │   │       │   │   ├── 0
    │   │       │   │   └── 1
    │   │       │   └── para
    │   │       ├── GlossSee
    │   │       ├── GlossTerm
    │   │       ├── ID
    │   │       └── SortAs
    │   └── title
    └── title

6 directories, 11 files
$ cat mnt/glossary/GlossDiv/GlossList/GlossEntry/Abbrev
ISO 8879:1986$ cat mnt/glossary/GlossDiv/GlossList/GlossEntry/Acronym 
SGML$ cat mnt/glossary/title 
example glossary$ cat json_eg1.json 
{
    "glossary": {
        "title": "example glossary",
		"GlossDiv": {
            "title": "S",
			"GlossList": {
                "GlossEntry": {
                    "ID": "SGML",
					"SortAs": "SGML",
					"GlossTerm": "Standard Generalized Markup Language",
					"Acronym": "SGML",
					"Abbrev": "ISO 8879:1986",
					"GlossDef": {
                        "para": "A meta-markup language, used to create markup languages such as DocBook.",
						"GlossSeeAlso": ["GML", "XML"]
                    },
					"GlossSee": "markup"
                }
            }
        }
    }
}
$ ps | grep ffs
80762 ttys001    0:00.03 ffs mnt json_eg1.json
80843 ttys001    0:00.00 grep ffs
$ umount mnt
[1]+  Done                    ffs mnt json_eg1.json
$
```

# External dependencies

You need an appropriate [FUSE](https://github.com/libfuse/libfuse) or
[macFUSE](https://osxfuse.github.io/) along with
[pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/).

See [the GitHub build
workflow](https://github.com/mgree/ffs/blob/main/.github/workflows/build.yml)
for examples of external dependency installation.

# TODO

- [x] `ListDirectory` (need element names, otherwise basically the same)
- [x] Settable mode
- [ ] Filenames
  + [x] Check on validity of filenames/fieldnames
  + [ ] Options for naming of ListDirectory elements
  + [ ] Metadata (as extensions or as dotfiles)
- [x] Debugging/logging
  + [ ] Instrument all `Filesystem` trait methods
  + [ ] Timing
  + [ ] Clean stderr output for `error!` and `warn!`
  + [ ] Quiet mode
- [ ] Writable FS
  + [x] rename
  + [x] rmdir
  + [x] access
  + [x] create
  + [x] fallocate
  + [ ] copy_file_range
- [ ] Output final FS to file at unmount
  + [ ] Choose target
  + [ ] fsync
- [ ] Other formats
  + [ ] TOML
  + [ ] XML
  + [ ] Generic framework (detect/parse/unparse)
- [ ] Missing tests
  + [ ] access
  + [ ] multi-user stuff
  + [ ] error code coverage
