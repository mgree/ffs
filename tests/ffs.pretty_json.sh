#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

ffs -m "$MNT" --target json -o "$OUT" --pretty ../json/object.json &
PID=$!
sleep 2

echo mgree >"$MNT"/handle

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

[ "$(cat $OUT | wc -l)" -eq 6 ] || fail lines
grep '^\s*"handle": "mgree",$' "$OUT" >/dev/null 2>&1 || fail handle

rmdir "$MNT" || fail mount
rm "$OUT"
