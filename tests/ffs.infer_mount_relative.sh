#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

TMP=$(mktemp -d)
MNT="$TMP/nested/object"

testcase_cleanup() { rm -rf "$TMP"; }

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
