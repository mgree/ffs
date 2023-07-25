#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$TGT"
        rm "$TGT2"
        rm "$ICO"
    fi
    exit 1
}

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

unpack --into "$MNT" ../json/object.json || fail unpack1

cp ../binary/twitter.ico "$MNT"/favicon
pack "$MNT" >"$TGT" || fail pack1
rm -r "$MNT"

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep favicon "$TGT" >/dev/null 2>&1 || fail text
unpack --into "$MNT" "$TGT" || fail unpack2

ICO=$(mktemp)

ls "$MNT" | grep favicon >/dev/null 2>&1 || fail field
decode "$MNT"/favicon "$ICO"
diff ../binary/twitter.ico "$ICO" || fail diff

pack --no-output "$MNT" >"$TGT2" || fail pack2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rm -r "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ICO"
