#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
SRC=$(mktemp)
TGT=$(mktemp)

testcase_cleanup() { rm -f "$SRC" "$TGT"; }

cp ../toml/single.toml "$SRC"

ffs --source toml --target json -o "$TGT" -m "$MNT" "$SRC" &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../json/single.json || fail diff

rmdir "$MNT" || fail mount
rm "$SRC" "$TGT"
