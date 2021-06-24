#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$TGT"
    fi
    exit 1
}

MNT=$(mktemp -d)
TGT=$(mktemp)

ffs --source json --target toml -o "$TGT" -m "$MNT" ../json/single.json &
PID=$!
sleep 2
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

diff "$TGT" ../toml/single.toml || fail diff

rmdir "$MNT" || fail mount
rm "$TGT"

