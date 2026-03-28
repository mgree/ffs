#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
EXP=$(mktemp -d)

testcase_cleanup() { rm -rf "$EXP"; }

# generate files w/newlines
printf "Michael Greenberg\n" >"${EXP}/name"
printf "2\n"                 >"${EXP}/eyes"
printf "10\n"                >"${EXP}/fingernails"
printf "true\n"              >"${EXP}/human"
printf ""                    >"${EXP}/problems"

ffs -m "$MNT" ../json/object_null.json &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
case $(ls) in
    (eyes*fingernails*human*name*problems) ;;
    (*) fail ls;;
esac
diff "${EXP}/name" "${MNT}/name" || fail name
diff "${EXP}/eyes" "${MNT}/eyes" || fail eyes
diff "${EXP}/fingernails" "${MNT}/fingernails" || fail fingernails
diff "${EXP}/human" "${MNT}/human" || fail human
diff "${EXP}/problems" "${MNT}/problems" || fail problems

cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
rm -r "$EXP"
