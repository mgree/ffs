#!/bin/sh
# Read every file in the mounted tree to /dev/null.
find "$1" -type f -exec cat {} \; > /dev/null
