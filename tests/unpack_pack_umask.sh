#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

umask 022
unpack --into "$MNT" ../json/object.json
cd "$MNT"
ls -l eyes | grep -e 'rw-r--r--' >/dev/null 2>&1 || fail file1
mkdir pockets
ls -ld pockets | grep -e 'rwxr-xr-x' >/dev/null 2>&1 || fail dir1
cd - >/dev/null 2>&1
pack "$MNT"
rm -r "$MNT"

umask 077
unpack --into "$MNT" ../json/object.json
cd "$MNT"
ls -l eyes | grep -e 'rw-------' >/dev/null 2>&1 || fail file2
mkdir pockets
ls -ld pockets | grep -e 'rwx------' >/dev/null 2>&1 || fail dir2
cd - >/dev/null 2>&1
pack "$MNT"

rm -r "$MNT" || fail mount
