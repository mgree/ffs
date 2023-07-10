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
mkdir pockets
case $(ls) in
    (eyes*fingernails*name*pockets) ;;
    (*) fail ls3;;
esac
rm pockets && fail rm1
case $(ls) in
    (eyes*fingernails*name*pockets) ;;
    (*) fail ls4;;
esac
echo keys >pockets/pants
rmdir pockets && fail rm2
rm pockets/pants
rmdir pockets || fail rmdir
case $(ls) in
    (eyes*fingernails*name) ;;
    (*) fail ls5;;
esac
cd - >/dev/null 2>&1

pack "$MNT"

rm -r "$MNT" || fail mount
