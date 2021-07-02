#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

printf '{".":"primo","..":"secondo","dot":"terzo","dotdot":"quarto"}' >"$EXP"

ffs -m "$MNT" -o "$OUT" --target json ../json/obj_rename.json &
PID=$!
sleep 2
case $(ls "$MNT") in
    (dot*dot_*dotdot*dotdot_) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/dot)" = "first" ] || fail dot
[ "$(cat $MNT/dotdot)" = "second" ] || fail dotdot
[ "$(cat $MNT/dot_)" = "third" ] || fail dot_
[ "$(cat $MNT/dotdot_)" = "fourth" ] || fail dotdot_

echo primo >"$MNT"/dot
echo secondo >"$MNT"/dotdot
echo terzo >"$MNT"/dot_
echo quarto >"$MNT"/dotdot_

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

diff "$OUT" "$EXP" || fail diff

rmdir "$MNT" || fail mount
rm "$OUT" "$EXP"
