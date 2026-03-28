#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)

cat ../json/object.json | ffs -m "$MNT" &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat name)" = "Michael Greenberg" ] || fail name
[ "$(cat eyes)" -eq 2 ] || fail eyes
[ "$(cat fingernails)" -eq 10 ] || fail fingernails
[ "$(cat human)" = "true" ] || fail human
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
