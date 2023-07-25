#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$MSG" "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)
MSG=$(mktemp)

unpack --into "$MNT" ../json/null.json >"$OUT" 2>"$MSG" || fail unpack

cat "$MSG" | grep -i -e  "must be a directory" >/dev/null 2>&1 || fail error
[ -f "$OUT" ] && ! [ -s "$OUT" ] || fail output

pack "$MNT" || fail pack

rm -r "$MNT" || fail mount
rm "$MSG" "$OUT"
