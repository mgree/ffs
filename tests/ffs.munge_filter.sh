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

ffs -m "$MNT" --munge filter ../json/obj_rename.json &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (dot*dotdot) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/dot)" = "third" ] || fail dot
[ "$(cat $MNT/dotdot)" = "fourth" ] || fail dotdot
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
