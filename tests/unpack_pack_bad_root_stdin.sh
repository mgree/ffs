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

echo \"just a string\" | unpack --into "$MNT" >"$OUT" 2>"$MSG" && fail "unpack error"

cat "$MSG" | grep -i -e "must be a directory" >/dev/null 2>&1 || fail error
[ -f "$OUT" ] && ! [ -s "$OUT" ] || fail output

rm -r "$MNT" || fail mount
rm "$MSG" "$OUT"
