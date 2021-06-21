#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$MSG" "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)
MSG=$(mktemp)

ffs "$MNT" ../json/null.json >"$OUT" 2>"$MSG" &
PID=$!
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process
cat "$MSG" | grep -i -e  "must be a directory" >/dev/null 2>&1 || fail error
[ -f "$OUT" ] && ! [ -s "$OUT" ] || fail output
sleep 1

rmdir "$MNT" || fail mount
rm "$MSG" "$OUT"
