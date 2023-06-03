#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm -r "$EXP"
        rm "$JSON"
    fi
    exit 1
}

MNT=$(mktemp -d)
EXP=$(mktemp -d)
JSON=$(mktemp)

# generate files w/newlines
printf "Michael Greenberg" >"${EXP}/name"
printf "2"                 >"${EXP}/eyes"
printf "10"                >"${EXP}/fingernails"
printf "true"              >"${EXP}/human"
printf "hi"                >"${EXP}/greeting"
printf "bye"               >"${EXP}/farewell"

unpack --into "$MNT" ../json/object.json

echo hi >"$MNT"/greeting
printf "bye" >"$MNT"/farewell

pack -o "$JSON" "$MNT"
rm -r "$MNT"

# remount w/ --exact, confirm that they're not there
unpack --exact --into "$MNT" "$JSON"

case $(ls "$MNT") in
    (eyes*farewell*fingernails*greeting*human*name) ;;
    (*) fail ls;;
esac
for x in "$EXP"/*
do
    diff "$x" "$MNT/$(basename $x)" || fail "$(basename $x)"
done

rm -r "$MNT" || fail mount
rm -r "$EXP"
