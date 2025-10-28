#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs -m "$MNT" --eager ../json/json_eg1.json &
PID=$!
sleep 2
case $(ls "$MNT") in
    (glossary) ;;
    (*) fail ls;;
esac
umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount

