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
        rm "$ICO"
    fi
    exit 1
}

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)

ffs "$MNT" ../json/object.json >"$TGT" &
PID=$!
sleep 2
cp ../binary/twitter.ico "$MNT"/favicon
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep favicon "$TGT" >/dev/null 2>&1 || fail text
ffs --no-output "$MNT" "$TGT" >"$TGT2" &
PID=$!
sleep 2

ICO=$(mktemp)

ls "$MNT" | grep favicon >/dev/null 2>&1 || fail field
base64 -D -i "$MNT"/favicon -o "$ICO"
diff ../binary/twitter.ico "$ICO" || fail diff

umount "$MNT" || fail unmount2
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ICO"
