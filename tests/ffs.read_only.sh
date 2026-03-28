#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)

ffs --readonly -m "$MNT" ../json/object.json &
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
[ -e touch ] && fail touch
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls2;;
esac
mkdir recipes
[ -e recipes ] && fail mkdir
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls3;;
esac
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
