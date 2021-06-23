#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$ERR"
    fi
    exit 1
}

MNT=$(mktemp -d)
ERR=$(mktemp)

ffs --no-output "$MNT" ../json/object.json &
PID=$!
sleep 2
touch "$MNT"/name 2>$ERR >&2 || fail touch
[ -s "$ERR" ] && { cat "$ERR"; fail error ; }
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

rmdir "$MNT" || fail mount
rm "$ERR"

