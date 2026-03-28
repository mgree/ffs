#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

D=$(mktemp -d)

MNT="$D/foo"
OUT="$D/foo.json"

EXP=$(mktemp)

testcase_cleanup() { rm -f "$EXP"; rm -rf "$D"; }

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
