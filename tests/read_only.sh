#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs --readonly true "$MNT" ../json/object.json &
PID=$!
sleep 2
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
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
