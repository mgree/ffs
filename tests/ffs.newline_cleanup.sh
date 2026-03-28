#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm -r "$EXP"
        rm "$JSON"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

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

ffs -o "$JSON" -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
echo hi >"$MNT"/greeting
printf "bye" >"$MNT"/farewell
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

# remount w/ --exact, confirm that they're not there
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
