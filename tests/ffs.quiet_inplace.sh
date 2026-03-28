#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$JSON" "$LOG"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
JSON=$(mktemp)
LOG=$(mktemp)

cp ../json/object.json "$JSON"

ffs -qi -m "$MNT" "$JSON" >"$LOG" 2>&1 &
PID=$!
"$WAITFOR" mount "$MNT"
echo hi >"$MNT"/greeting
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process1

diff ../json/object.json "$JSON" >/dev/null && fail same
[ "$(cat $LOG )" = "" ] || fail quiet

ffs --readonly -m "$MNT" "$JSON" &
PID=$!
"$WAITFOR" mount "$MNT"
[ "$(cat $MNT/greeting)" = "hi" ] || fail updated
"$WAITFOR" umount "$MNT" || fail "$WAITFOR" umount2
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process2

rmdir "$MNT" || fail mount
rm "$JSON" || fail copy
