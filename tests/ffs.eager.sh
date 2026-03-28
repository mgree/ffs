#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

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

