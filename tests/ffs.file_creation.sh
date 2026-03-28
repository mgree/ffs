#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat name)" = "Michael Greenberg" ] || fail name
[ "$(cat eyes)" -eq 2 ] || fail eyes
[ "$(cat fingernails)" -eq 10 ] || fail fingernails
[ "$(cat human)" = "true" ] || fail human
touch jokes
[ -f jokes ] || fail touch
case $(ls) in
    (eyes*fingernails*human*jokes*name) ;;
    (*) fail ls2;;
esac
mkdir recipes
[ -d recipes ]|| fail mkdir
case $(ls) in
    (eyes*fingernails*human*jokes*name*recipes) ;;
    (*) fail ls3;;
esac
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
