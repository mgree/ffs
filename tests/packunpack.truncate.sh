#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
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

unpack --into "$MNT" ../json/object.json || fail unpack1
echo 'Mikey Indiana' >"$MNT"/name 2>"$ERR"
[ -s "$ERR" ] && fail non-empty error
pack "$MNT" >"$TGT" || fail pack1
rm -r "$MNT"

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
grep -e Indiana "$TGT" >/dev/null 2>&1 || fail grep
unpack --type json --into "$MNT" "$TGT" || fail unpack2

case $(ls "$MNT") in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac

[ "$(cat $MNT/name)" = "Mikey Indiana" ] || fail contents

pack --no-output "$MNT" >"$TGT2" || fail pack2

[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rm -r "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
rm "$ERR"
