#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

ffs -m "$MNT" --eager ../json/json_eg1.json &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (glossary) ;;
    (*) fail ls;;
esac
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

rmdir "$MNT" || fail mount
