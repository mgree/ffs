#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

testcase_cleanup() { rm -f "$OUT" "$EXP"; }

printf '{"he":{"dot":"shlishi"},"imnewhere":"derp","it":{".":"primo","..":"secondo"}}' >"$EXP"

ffs -m "$MNT" -o "$OUT" --target json ../json/obj_rename.json &
PID=$!
"$WAITFOR" mount "$MNT"
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

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

diff "$OUT" "$EXP" || fail diff

rmdir "$MNT" || fail mount
rm "$OUT" "$EXP"
