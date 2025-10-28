#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

unpack --into "$MNT" ../json/object.json || fail unpack

echo mgree >"$MNT"/handle

pack --target json -o "$OUT" --pretty "$MNT" || fail pack

[ "$(cat $OUT | wc -l)" -eq 6 ] || fail lines
grep '^\s*"handle": "mgree",$' "$OUT" >/dev/null 2>&1 || fail handle

rm -r "$MNT" || fail mount
rm "$OUT"
