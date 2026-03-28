#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
EXP=$(mktemp -d)
JSON=$(mktemp)

testcase_cleanup() { rm -rf "$EXP"; rm -f "$JSON"; }

# generate files w/newlines
printf "Michael Greenberg" >"${EXP}/name"
printf "2"                 >"${EXP}/eyes"
printf "10"                >"${EXP}/fingernails"
printf "true"              >"${EXP}/human"
printf "hi\n"              >"${EXP}/greeting"
printf "bye"               >"${EXP}/farewell"

ffs --exact -o "$JSON" -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
echo hi >"$MNT"/greeting
printf "bye" >"$MNT"/farewell
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

# remount w/ --exact, confirm that they're not there (except for greeting)
ffs --exact -m "$MNT" "$JSON" &
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (eyes*farewell*fingernails*greeting*human*name) ;;
    (*) fail ls;;
esac
for x in "$EXP"/*
do
    diff "$x" "$MNT/$(basename $x)" || fail "$(basename $x)"
done
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

rmdir "$MNT" || fail mount
rm -r "$EXP"
rm -f "$JSON"
