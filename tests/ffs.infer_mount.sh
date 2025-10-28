#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$TMP"/object
        rm -r "$TMP"
    fi
    exit 1
}

TMP=$(mktemp -d)

cp ../json/object.json "$TMP"
cd "$TMP"
ffs object.json &
PID=$!
sleep 2
[ -d "object" ] || fail mountdir
case $(ls object) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat object/name)" = "Michael Greenberg" ] || fail name
[ "$(cat object/eyes)" -eq 2 ] || fail eyes
[ "$(cat object/fingernails)" -eq 10 ] || fail fingernails
[ "$(cat object/human)" = "true" ] || fail human
umount object || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

[ -d "object" ] && fail cleanup
cd -
rm -r "$TMP"
