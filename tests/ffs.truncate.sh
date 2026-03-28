#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)
ERR=$(mktemp)

testcase_cleanup() { rm -f "$TGT" "$TGT2" "$ERR"; }

ffs -m "$MNT" ../json/object.json >"$TGT" &
PID=$!
"$WAITFOR" mount "$MNT"
echo 'Mikey Indiana' >"$MNT"/name 2>"$ERR"
[ -s "$ERR" ] && fail non-empty error
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit "$PID"
kill -0 $PID >/dev/null 2>&1 && fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep -e Indiana "$TGT" >/dev/null 2>&1 || fail grep
ffs --no-output --source json -m "$MNT" "$TGT" >"$TGT2" &
PID=$!
"$WAITFOR" mount "$MNT"

case $(ls "$MNT") in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac

[ "$(cat $MNT/name)" = "Mikey Indiana" ] || fail contents

"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit "$PID"
kill -0 $PID >/dev/null 2>&1 && fail process2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ERR"
