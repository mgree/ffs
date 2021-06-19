#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$MSG"
    fi
    exit 1
}

MNT=$(mktemp -d)
MSG=$(mktemp)

ffs "$MNT" ../json/null.json 2>"$MSG" &
PID=$!
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process
cat "$MSG" | grep -i -e error >/dev/null 2>&1 || fail error
sleep 1

rmdir "$MNT" || fail mount
rm "$MSG"
