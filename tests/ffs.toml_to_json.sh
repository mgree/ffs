#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
TGT=$(mktemp)

testcase_cleanup() { rm -f "$TGT"; }

ffs --source toml --target json -o "$TGT" -m "$MNT" ../toml/single.toml &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../json/single.json || fail diff

rmdir "$MNT" || fail mount
rm "$TGT"
