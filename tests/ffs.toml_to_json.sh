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

ffs --source toml --target json -o "$TGT" -m "$MNT" ../toml/single.toml &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../json/single.json || fail diff

rmdir "$MNT" || fail mount
rm "$TGT"

