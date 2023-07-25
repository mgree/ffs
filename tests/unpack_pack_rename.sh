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

unpack --into "$MNT" ../json/object.json || fail unpack

cd "$MNT"
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls1;;
esac
mv name full_name
[ "$(cat full_name)" = "Michael Greenberg" ] || fail name1
case $(ls) in
    (eyes*fingernails*full_name*human) ;;
    (*) fail ls2;;
esac
echo Prof. G >name
mv full_name name
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls3;;
esac
[ "$(cat name)" = "Michael Greenberg" ] || fail name2
mv nonesuch name && fail mv1
case $(ls) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls4;;
esac
cd - >/dev/null 2>&1

pack "$MNT" || fail pack

rm -r "$MNT" || fail mount
