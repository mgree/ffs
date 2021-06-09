#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs "$MNT" ../json/list.json &
PID=$!
sleep 1
cd "$MNT"
case $(ls) in
    (0*1*2*3) ;;
    (*) fail ls;;
esac
[ "$(cat 0)" -eq 1 ] || fail 0
[ "$(cat 1)" -eq 2 ] || fail 1
[ "$(cat 2)" = "3" ] || fail 2
[ "$(cat 3)" = "false" ] || fail 3
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
