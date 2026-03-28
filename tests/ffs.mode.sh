#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

umask 022

ffs --mode 666 -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
ls -l eyes | grep -e 'rw-rw-rw-' >/dev/null 2>&1 || fail file1
mkdir pockets
ls -ld pockets | grep -e 'rwxr-xr-x' >/dev/null 2>&1 || fail dir1
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

umask 077
ffs --mode 666 --dirmode 700 -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
ls -l eyes | grep -e 'rw-rw-rw-' >/dev/null 2>&1 || fail file2
mkdir pockets
ls -ld pockets | grep -e 'rwx----' >/dev/null 2>&1 || fail dir2
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2


rmdir "$MNT" || fail mount
