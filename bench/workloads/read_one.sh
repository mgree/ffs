#!/bin/sh
# Read one root-level file to /dev/null.
find "$1" -maxdepth 1 -type f | head -1 | xargs -I{} cat {} > /dev/null
