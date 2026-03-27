#!/bin/sh
# Sum all leaf files whose content is a bare number; print total to stdout.
find "$1" -type f -exec cat {} \; | awk '/^-?[0-9]+\.?[0-9]*$/ { sum += $1 } END { print sum+0 }'
