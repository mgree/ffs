#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$TMP"/nested/object
        rm -r "$TMP"
    fi
    exit 1
}

TMP=$(mktemp -d)

cp ../json/object.json "$TMP"
mkdir "$TMP"/nested
cd "$TMP"/nested

unpack ../object.json || fail unpack

[ -d "object" ] || fail mountdir
case $(ls object) in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac
[ "$(cat object/name)" = "Michael Greenberg" ] || fail name
[ "$(cat object/eyes)" -eq 2 ] || fail eyes
[ "$(cat object/fingernails)" -eq 10 ] || fail fingernails
[ "$(cat object/human)" = "true" ] || fail human

rm -r "$TMP"/nested/object

[ -d "object" ] && fail cleanup
cd - >/dev/null 2>&1
pack "$MNT" || fail pack
rm -r "$TMP"
