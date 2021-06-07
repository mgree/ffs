# ffs: the file filesystem

Working with semi-structured data using command-line tools is hard!
Tools like [jq](https://github.com/stedolan/jq) help a lot, but
learning a new language for simple manipulations is a big ask.

ffs, the **f**ile **f**ile**s**sytem, let's you mount semi-structured
data as a fileystem---a tree structure you already know how to work with!

# External dependencies

You need a form of FUSE and
[pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/).

## Installation on macOS

You need to have [macFUSE](https://osxfuse.github.io/) installed to
run on macOS.

```shell-session
$ brew install macfuse
$ reboot
$ cd /usr/local/lib/pkgconfig
$ sudo ln -sf fuse.pc osxfuse.pc
```
