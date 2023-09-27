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

unpack --into "$MNT" ../json/list.json || fail unpack

cd "$MNT"
case $(ls) in
    (0*1*2*3) ;;
    (*) fail ls;;
esac
[ "$(cat 0)" -eq 1 ] || fail 0
[ "$(cat 1)" -eq 2 ] || fail 1
[ "$(cat 2)" = "3" ] || fail 2
[ "$(cat 3)" = "false" ] || fail 3
cd - >/dev/null 2>&1

pack "$MNT" || fail pack
rm -r "$MNT" || fail mount
