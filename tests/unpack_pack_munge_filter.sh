#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

unpack --into "$MNT" --munge filter ../json/obj_rename.json

case $(ls "$MNT") in
    (dot*dotdot) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/dot)" = "third" ] || fail dot
[ "$(cat $MNT/dotdot)" = "fourth" ] || fail dotdot

rm -r "$MNT" || fail mount
