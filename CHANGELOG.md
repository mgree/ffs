# ffs - Changelog

## 0.1.2 - 2021-09-26

* Handle failed mounts better, with an appropriate message and error
  code.
* Revise exit codes: 0 means success, 1 means FS error, 2 means CLI
  error.
* `--time` flag for emitting timing information on STDERR.
* Basic startup/shutdown benchmarking, with microbenchmarks.
* Upgrade to fuser 0.9.1. Concomitant bugfixes turn off AutoUnmount.
* Improvements/bugfixes in regression tests.

## 0.1.1 - 2021-07-15

* Extended attribute `user.type` manages metadata.
* Ignore macOS extended attribute files `._*`.
* `--pretty` flag for JSON and TOML.
* Wrote INSTALL.md.
* Improved manpage.
* `--new` flag for starting from an empty filesystem.
* `--munge` flag for controlling renaming; revised renaming
  policy. Restore files whose names are munged.

## 0.1.0 - 2021-06-26

Initial release.
