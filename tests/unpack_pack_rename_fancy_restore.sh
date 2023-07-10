#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

printf '{"he":{"dot":"shlishi"},"imnewhere":"derp","it":{".":"primo","..":"secondo"}}' >"$EXP"

unpack --into "$MNT" ../json/obj_rename.json

case $(ls "$MNT") in
    (_.*_..*dot*dotdot) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/_.)" = "first" ] || fail .
[ "$(cat $MNT/_..)" = "second" ] || fail ..
[ "$(cat $MNT/dot)" = "third" ] || fail dot
[ "$(cat $MNT/dotdot)" = "fourth" ] || fail dotdot

echo primo >"$MNT"/_.
echo secondo >"$MNT"/_..
echo shlishi >"$MNT"/dot
echo derp >"$MNT"/dotdot

mkdir "$MNT"/it
mkdir "$MNT"/he

mv "$MNT"/_.    "$MNT"/it
mv "$MNT"/_.. "$MNT"/it

mv "$MNT"/dot    "$MNT"/he

mv "$MNT"/dotdot "$MNT"/imnewhere

pack --target json -o "$OUT" "$MNT"

diff "$OUT" "$EXP" || fail diff

rm -r "$MNT" || fail mount
rm "$OUT" "$EXP"
