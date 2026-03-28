#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$TGT"
        rm "$TGT2"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)

ffs -m "$MNT" ../json/object.json >"$TGT" &
PID=$!
"$WAITFOR" mount "$MNT"
echo 'echo hi' >"$MNT"/script
chmod +x "$MNT"/script
[ "$($MNT/script)" = "hi" ] || fail exec
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep -e echo "$TGT" >/dev/null 2>&1 || fail grep
ffs --no-output --source json -m "$MNT" "$TGT" >"$TGT2" &
PID=$!
"$WAITFOR" mount "$MNT"

case $(ls "$MNT") in
    (eyes*fingernails*human*name*script) ;;
    (*) fail ls;;
esac

[ "$(cat $MNT/script)" = "echo hi" ] || fail contents

"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
