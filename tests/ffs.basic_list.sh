#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/list.json &
PID=$!
"$WAITFOR" mount "$MNT"
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
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
