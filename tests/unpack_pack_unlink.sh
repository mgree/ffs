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

unpack --into "$MNT" ../json/object.json

cd "$MNT"
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat name)" = "Michael Greenberg" ] || fail name
[ "$(cat eyes)" -eq 2 ] || fail eyes
[ "$(cat fingernails)" -eq 10 ] || fail fingernails
[ "$(cat human)" = "true" ] || fail human1
rm human
case $(ls) in
    (eyes*fingernails*name) ;;
    (*) fail ls2;;
esac
echo false >human
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls3;;
esac
[ "$(cat human)" = "false" ] || fail human2
cd - >/dev/null 2>&1
pack "$MNT"

rm -r "$MNT" || fail mount