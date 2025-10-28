#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rm -r "$D"
    fi
    exit 1
}

D=$(mktemp -d)

MNT=foo
OUT=foo.json

EXP=$(mktemp)

printf '{"handles":{"github":"mgree","stevens":"mgreenbe","twitter":"mgrnbrg"},"problems":99}' >"$EXP"

cd "$D"
ffs --new "$OUT" &
PID=$!
sleep 2
[ "$(ls $MNT)" ] && fail nonempty

mkdir "$MNT"/handles

echo mgree    >"$MNT"/handles/github
echo mgreenbe >"$MNT"/handles/stevens
echo mgrnbrg  >"$MNT"/handles/twitter
echo 99       >"$MNT"/problems

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

diff "$OUT" "$EXP" || fail diff

[ -e "$MNT" ] && fail mount
rm -r "$D"
