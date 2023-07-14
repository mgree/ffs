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

unpack --into "$MNT" ../json/object.json
mkdir "$MNT"/pockets
echo keys >"$MNT"/pockets/pants
echo pen >"$MNT"/pockets/shirt
cd - >/dev/null 2>&1
pack "$MNT" >"$TGT"
rm -r "$MNT"

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
[ -s "$TGT" ] || fail output2
cat "$TGT"
stat "$TGT"
unpack --into "$MNT" "$TGT"

case $(ls "$MNT") in
    (eyes*fingernails*human*name*pockets) ;;
    (*) fail ls1;;
esac
case $(ls "$MNT"/pockets) in
    (pants*shirt) ;;
    (*) fail ls2;;
esac

[ "$(cat $MNT/name)" = "Michael Greenberg" ] || fail name
[ "$(cat $MNT/eyes)" -eq 2 ] || fail eyes
[ "$(cat $MNT/fingernails)" -eq 10 ] || fail fingernails
[ "$(cat $MNT/human)" = "true" ] || fail human
[ "$(cat $MNT/pockets/pants)" = "keys" ] || fail pants
[ "$(cat $MNT/pockets/shirt)" = "pen" ] || fail shirt

pack --no-output "$MNT" >"$TGT2"

stat "$TGT2"
[ -f "$TGT2" ] || fail tgt2
[ -s "$TGT2" ] && fail tgt2_nonempty

rm -r "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
