#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$SRC" "$TGT"
    fi
    exit 1
}

MNT=$(mktemp -d)
SRC=$(mktemp)
TGT=$(mktemp)

cp ../toml/single.toml "$SRC"

unpack --type toml --into "$MNT" "$SRC"

pack --target json -o "$TGT" "$MNT"

diff "$TGT" ../json/single.json || fail diff

rm -r "$MNT" || fail mount
rm "$SRC" "$TGT"

