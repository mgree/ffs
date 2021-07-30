To run tests, run `run_tests.sh` (in the repo root).

These tests cover a wide variety of features. Testing is slow because
mountpoints aren't _immediately_ available after running `ffs` in the
background---you need a few milliseconds, but there's no portable way
to sleep just a little.
