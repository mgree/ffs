# ffs: the file filesystem

Working with semi-structured data using command-line tools is hard!
Tools like [jq](https://github.com/stedolan/jq) help a lot, but
learning a new language for simple manipulations is a big ask.

ffs, the **f**ile **f**ile**s**sytem, let's you mount semi-structured
data as a fileystem---a tree structure you already know how to work with!

# External dependencies

You need an appropriate [FUSE](https://github.com/libfuse/libfuse) or
[macFUSE](https://osxfuse.github.io/) along with
[pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/).

See [the GitHub build
workflow](https://github.com/mgree/ffs/blob/main/.github/workflows/build.yml)
for examples of external dependency installation.
