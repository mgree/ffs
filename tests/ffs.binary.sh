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
        rm "$ICO"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    decode() {
        base64 -d $1 >$2
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    decode() {
        base64 -D -i $1 -o $2
    }
else
    fail os
fi

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)

ffs -m "$MNT" ../json/object.json >"$TGT" &
PID=$!
"$WAITFOR" mount "$MNT"
cp ../binary/twitter.ico "$MNT"/favicon
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep favicon "$TGT" >/dev/null 2>&1 || fail text
ffs --no-output -m "$MNT" "$TGT" >"$TGT2" &
PID=$!
"$WAITFOR" mount "$MNT"

ICO=$(mktemp)

ls "$MNT" | grep favicon >/dev/null 2>&1 || fail field
decode "$MNT"/favicon "$ICO"
diff ../binary/twitter.ico "$ICO" || fail diff

"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ICO"
