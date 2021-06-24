#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
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

ffs --newline -o "$JSON" -m "$MNT" ../json/object.json &
PID=$!
sleep 2
echo hi >"$MNT"/greeting
printf "bye" >"$MNT"/farewell
umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

# remount w/o --newline, confirm that they're not there
ffs -m "$MNT" "$JSON" &
sleep 2
case $(ls "$MNT") in
    (eyes*farewell*fingernails*greeting*human*name) ;;
    (*) fail ls;;
esac
for x in "$EXP"/*
do
    diff "$x" "$MNT/$(basename $x)" || fail "$(basename $x)"
done
umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
rm -r "$EXP"
