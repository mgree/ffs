#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
OUT=$(mktemp)
MSG=$(mktemp)

testcase_cleanup() { rm -f "$OUT" "$MSG"; }

ffs -m "$MNT" ../json/null.json >"$OUT" 2>"$MSG" &
PID=$!
"$WAITFOR" exit $PID || fail process
cat "$MSG" | grep -i -e  "must be a directory" >/dev/null 2>&1 || fail error
[ -f "$OUT" ] && ! [ -s "$OUT" ] || fail output

rmdir "$MNT" || fail mount
rm "$MSG" "$OUT"
