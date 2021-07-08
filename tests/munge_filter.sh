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

ffs -m "$MNT" --munge filter ../json/obj_rename.json &
PID=$!
sleep 2
case $(ls "$MNT") in
    (dot*dotdot) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/dot)" = "third" ] || fail dot
[ "$(cat $MNT/dotdot)" = "fourth" ] || fail dotdot
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
