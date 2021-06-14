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

ffs "$MNT" ../json/obj_rename.json &
PID=$!
sleep 2
cd "$MNT"
case $(ls) in
    (dot*dot_*dotdot*dotdot_) ;;
    (*) fail ls;;
esac
[ "$(cat dot)" = "first" ] || fail dot
[ "$(cat dotdot)" = "second" ] || fail dotdot
[ "$(cat dot_)" = "third" ] || fail dot_
[ "$(cat dotdot_)" = "fourth" ] || fail dotdot_
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
