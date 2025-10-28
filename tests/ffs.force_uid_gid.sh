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

ffs --uid $(id -u root) --gid $(id -g root) -m "$MNT" ../json/object.json &
PID=$!
sleep 2
ls -l "$MNT" | grep root >/dev/null 2>&1 || fail user
ls -l "$MNT" | grep $(groups root | cut -d' ' -f 1) >/dev/null 2>&1 || fail group
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
