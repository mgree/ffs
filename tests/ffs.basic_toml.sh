#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

ffs -m "$MNT" ../toml/eg.toml &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (clients*database*owner*servers*title) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/title)" = "TOML Example" ] || fail title
[ "$(cat $MNT/owner/dob)" = "1979-05-27T07:32:00-08:00" ] || fail dob

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
