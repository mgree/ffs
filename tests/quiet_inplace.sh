#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)
JSON=$(mktemp)

cp ../json/object.json "$JSON"

ffs -qi -m "$MNT" "$JSON" &
PID=$!
sleep 2
echo hi >"$MNT"/greeting
umount "$MNT" || fail unmount1
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process1

diff ../json/object.json "$JSON" >/dev/null && fail same

ffs --readonly -m "$MNT" "$JSON" &
PID=$!
sleep 2
[ "$(cat $MNT/greeting)" = "hi" ] || fail updated
umount "$MNT" || fail umount2
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process2

rmdir "$MNT" || fail mount
rm "$JSON" || fail copy
