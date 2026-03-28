#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$SRC" "$TGT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
SRC=$(mktemp)
TGT=$(mktemp)

cp ../toml/single.toml "$SRC"

ffs --source toml --target json -o "$TGT" -m "$MNT" "$SRC" &
PID=$!
"$WAITFOR" mount "$MNT"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

diff "$TGT" ../json/single.json || fail diff

rmdir "$MNT" || fail mount
rm "$SRC" "$TGT"

