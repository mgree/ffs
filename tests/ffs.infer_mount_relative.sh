#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$TMP"/nested/object
        rm -r "$TMP"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

TMP=$(mktemp -d)

cp ../json/object.json "$TMP"
mkdir "$TMP"/nested
cd "$TMP"/nested

ffs ../object.json &
PID=$!
"$WAITFOR" mount object
[ -d "object" ] || fail mountdir
case $(ls object) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat object/name)" = "Michael Greenberg" ] || fail name
[ "$(cat object/eyes)" -eq 2 ] || fail eyes
[ "$(cat object/fingernails)" -eq 10 ] || fail fingernails
[ "$(cat object/human)" = "true" ] || fail human
"$WAITFOR" umount object || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

[ -d "object" ] && fail cleanup
cd -
rm -r "$TMP"
