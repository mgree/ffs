#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$ERR"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
ERR=$(mktemp)

ffs --no-output -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
touch "$MNT"/name 2>$ERR >&2 || { cat "$ERR"; fail touch; }
[ -s "$ERR" ] && { cat "$ERR"; fail error ; }
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

rmdir "$MNT" || fail mount
rm "$ERR"

