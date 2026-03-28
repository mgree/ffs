#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

printf '{".":"primo","..":"secondo","dot":"terzo","dotdot":"quarto"}' >"$EXP"

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
echo terzo >"$MNT"/dot
echo quarto >"$MNT"/dotdot

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

diff "$OUT" "$EXP" || fail diff

rmdir "$MNT" || fail mount
rm "$OUT" "$EXP"
