#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$MSG" "$OUT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
OUT=$(mktemp)
MSG=$(mktemp)

ffs -m "$MNT" ../json/null.json >"$OUT" 2>"$MSG" &
PID=$!
"$WAITFOR" exit $PID || fail process
cat "$MSG" | grep -i -e  "must be a directory" >/dev/null 2>&1 || fail error
[ -f "$OUT" ] && ! [ -s "$OUT" ] || fail output

rmdir "$MNT" || fail mount
rm "$MSG" "$OUT"
