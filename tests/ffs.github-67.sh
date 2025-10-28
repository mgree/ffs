#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rm "$OUT" "$SORTED_OUT" "$SORTED_ORIG"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

ffs -m "$MNT" -o "$OUT" ../toml/github-67.toml &
PID=$!
sleep 2

umount "$MNT" || fail unmount

SORTED_OUT=$(mktemp)
SORTED_ORIG=$(mktemp)

sort "$OUT" >"$SORTED_OUT"
sort ../toml/github-67.toml >"$SORTED_ORIG"

diff -w "$SORTED_ORIG" "$SORTED_OUT" || fail diff

rm "$OUT" "$SORTED_OUT" "$SORTED_ORIG"
rmdir "$MNT"
