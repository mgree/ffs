#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rm -r "$D"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

D=$(mktemp -d)

MNT=foo
OUT=foo.json

EXP=$(mktemp)

printf '{"handles":{"github":"mgree","stevens":"mgreenbe","twitter":"mgrnbrg"},"problems":99}' >"$EXP"

cd "$D"
ffs --new "$OUT" &
PID=$!
"$WAITFOR" mount "$MNT"
[ "$(ls $MNT)" ] && fail nonempty

mkdir "$MNT"/handles

echo mgree    >"$MNT"/handles/github
echo mgreenbe >"$MNT"/handles/stevens
echo mgrnbrg  >"$MNT"/handles/twitter
echo 99       >"$MNT"/problems

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

diff "$OUT" "$EXP" || fail diff

[ -e "$MNT" ] && fail mount
rm -r "$D"
