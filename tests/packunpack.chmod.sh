#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$TGT"
        rm "$TGT2"
    fi
    exit 1
}

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)

unpack --into "$MNT" ../json/object.json || fail unpack1

echo 'echo hi' >"$MNT"/script
chmod +x "$MNT"/script
[ "$($MNT/script)" = "hi" ] || fail exec

pack "$MNT" >"$TGT" || fail pack1
rm -r "$MNT"

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep -e echo "$TGT" >/dev/null 2>&1 || fail grep
unpack --type json --into "$MNT" "$TGT" || fail unpack2

case $(ls "$MNT") in
    (eyes*fingernails*human*name*script) ;;
    (*) fail ls;;
esac

[ "$(cat $MNT/script)" = "echo hi" ] || fail contents

pack --no-output >"$TGT2" "$MNT" || fail pack2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rm -r "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
