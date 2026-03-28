#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
TGT=$(mktemp)

testcase_cleanup() { rm -f "$TGT"; }

ffs --source json --target toml -o "$TGT" -m "$MNT" ../json/single.json &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../toml/single.toml || fail diff

rmdir "$MNT" || fail mount
rm "$TGT"
