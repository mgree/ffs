#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
OUT=$(mktemp)

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
