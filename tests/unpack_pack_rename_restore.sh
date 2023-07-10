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

printf '{".":"primo","..":"secondo","dot":"terzo","dotdot":"quarto"}' >"$EXP"

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
echo terzo >"$MNT"/dot
echo quarto >"$MNT"/dotdot

pack -o "$OUT" --target json "$MNT"

diff "$OUT" "$EXP" || fail diff

rm -r "$MNT" || fail mount
rm "$OUT" "$EXP"
