#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$ERR"
    fi
    exit 1
}

MNT=$(mktemp -d)
ERR=$(mktemp)

unpack --into "$MNT" ../json/object.json

touch "$MNT"/name 2>"$ERR" >&2 || { cat "$ERR"; fail touch; }
[ -s "$ERR" ] && { cat "$ERR"; fail error ; }

rm -r "$MNT" || fail mount
rm "$ERR"

