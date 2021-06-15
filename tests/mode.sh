#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs --mode 666 "$MNT" ../json/object.json &
PID=$!
sleep 2
cd "$MNT"
ls -l eyes | grep -e 'rw-rw-rw-' >/dev/null 2>&1 || fail file1
mkdir pockets
ls -ld pockets | grep -e 'rwxrwxrwx' >/dev/null 2>&1 || fail dir1
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount1
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

ffs --mode 666 --dirmode 700 "$MNT" ../json/object.json &
PID=$!
sleep 2
cd "$MNT"
ls -l eyes | grep -e 'rw-rw-rw-' >/dev/null 2>&1 || fail file2
mkdir pockets
ls -ld pockets | grep -e 'rwx------' >/dev/null 2>&1 || fail dir2
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount2
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process2


rmdir "$MNT" || fail mount
