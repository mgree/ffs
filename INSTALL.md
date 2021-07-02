`ffs` depends on FUSE, and is currently tested on Linux and macOS.

# Installing FUSE

On Linux, run `sudo apt-get install fuse` to install
[FUSE](https://github.com/libfuse/libfuse).

On macOS, run `brew install macfuse` to install
[macFUSE](https://osxfuse.github.io/).

# Installing `ffs`

The easiest way to install `ffs` is to grab a [release
binary](https://github.com/mgree/ffs/releases). There are two classes
of release: numbered releases and latest. Numbered releases are
arbitrary stable milestones; latest releases are automatically
generated builds from
[`main`](https://github.com/mgree/ffs/tree/main/).

You might also want to get the
[manpage](https://raw.githubusercontent.com/mgree/ffs/main/man/ffs.1)
and install it in an appropriate place.

## Source installations

You can also install it from source. `ffs` is written in Rust, so
you'll need to have a Rust compiler to hand. You'll need to make sure
you have `pkg-config` installed; on Linux, you will also need
`libfuse-dev`.

You then have two options: you can run `cargo install ffs` to get the
last numbered release, or you can build entirely locally to use the
latest build:

```shell-session
$ git clone https://github.com/mgree/ffs
$ cargo install --path .
```
