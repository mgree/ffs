#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

ffs --uid $(id -u root) --gid $(id -g root) -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
ls -l "$MNT" | grep root >/dev/null 2>&1 || fail user
ls -l "$MNT" | grep $(groups root | cut -d' ' -f 1) >/dev/null 2>&1 || fail group
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
