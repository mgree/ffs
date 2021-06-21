#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$SRC" "$TGT"
    fi
    exit 1
}

MNT=$(mktemp -d)
SRC=$(mktemp)
TGT=$(mktemp)

cp ../toml/single.toml "$SRC"

ffs --source toml --target json -o "$TGT" "$MNT" "$SRC" &
PID=$!
sleep 2
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

diff "$TGT" ../json/single.json || fail diff

rmdir "$MNT" || fail mount
rm "$SRC" "$TGT"

