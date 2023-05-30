#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        # cd
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
printf "hi\n"              >"${EXP}/greeting"
printf "bye"               >"${EXP}/farewell"

unpack --exact --into "$MNT" ../json/object.json

echo hi >"$MNT"/greeting
printf "bye" >"$MNT"/farewell

pack --exact "$MNT" -o "$JSON"
# TODO (nad) 2023-05-30: since there is no unmounting, i have to clear directory by removing it then recreating it
# check if there is a better way to do this
rm -r "$MNT" || fail unmount
mkdir "$MNT"

# remount w/ --exact, confirm that they're not there (except for greeting)
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
