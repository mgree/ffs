#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
ERR=$(mktemp)

testcase_cleanup() { rm -f "$ERR"; }

ffs --no-output -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
touch "$MNT"/name 2>$ERR >&2 || { cat "$ERR"; fail touch; }
[ -s "$ERR" ] && { cat "$ERR"; fail error ; }
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

rmdir "$MNT" || fail mount
rm "$ERR"
