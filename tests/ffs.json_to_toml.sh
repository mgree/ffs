#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$TGT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
TGT=$(mktemp)

ffs --source json --target toml -o "$TGT" -m "$MNT" ../json/single.json &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../toml/single.toml || fail diff

rmdir "$MNT" || fail mount
rm "$TGT"

