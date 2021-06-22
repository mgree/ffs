#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$TGT"
        rm "$TGT2"
        rm "$ERR"
    fi
    exit 1
}

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)
ERR=$(mktemp)

ffs "$MNT" ../json/object.json >"$TGT" &
PID=$!
sleep 2
echo 'Mikey Indiana' >"$MNT"/name 2>"$ERR"
[ -s "$ERR" ] && fail non-empty error
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep -e Indiana "$TGT" >/dev/null 2>&1 || fail grep
ffs --no-output --source json "$MNT" "$TGT" >"$TGT2" &
PID=$!
sleep 2

case $(ls "$MNT") in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac

[ "$(cat $MNT/name)" = "Mikey Indiana" ] || fail contents

umount "$MNT" || fail unmount2
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ERR"
