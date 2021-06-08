# ffs: the file filesystem
[![Main workflow](https://github.com/mgree/ffs/actions/workflows/build.yml/badge.svg)](https://github.com/mgree/ffs/actions/workflows/build.yml)

ffs, the **f**ile **f**ile**s**sytem, let's you mount semi-structured
data as a fileystem---a tree structure you already know how to work with!

Working with semi-structured data using command-line tools is hard.
Tools like [jq](https://github.com/stedolan/jq) help a lot, but
learning a new language for simple manipulations is a big ask. By mapping
hard-to-parse trees into a filesystem, you can keep using the tools you
know.

# External dependencies

You need an appropriate [FUSE](https://github.com/libfuse/libfuse) or
[macFUSE](https://osxfuse.github.io/) along with
[pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/).

See [the GitHub build
workflow](https://github.com/mgree/ffs/blob/main/.github/workflows/build.yml)
for examples of external dependency installation.

# TODO

- [ ] `ListDirectory` (need element names, otherwise basically the same)
- [ ] Check on validity of filenames/fieldnames
- [ ] Metadata (as extensions or as dotfiles)
- [ ] Debugging/logging
- [ ] Timing
- [ ] Writing, JSON output
