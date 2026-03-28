#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
OUT=$(mktemp)

testcase_cleanup() { rm -f "$OUT"; }

ffs -m "$MNT" --target json -o "$OUT" --pretty ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

echo mgree >"$MNT"/handle

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

[ "$(cat $OUT | wc -l)" -eq 6 ] || fail lines
grep '^\s*"handle": "mgree",$' "$OUT" >/dev/null 2>&1 || fail handle

rmdir "$MNT" || fail mount
rm "$OUT"
