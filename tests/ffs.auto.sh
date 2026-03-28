#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
FILE=$(mktemp).json

echo '{}' >"$FILE"

EXP=$(mktemp)

testcase_cleanup() { rm -f "$FILE" "$EXP"; }

printf '{"favorite_number":47,"likes":{"cats":false,"dogs":true},"mistakes":null,"name":"Michael Greenberg","website":"https://mgree.github.io"}' >"$EXP"

ffs -m "$MNT" -i "$FILE" &
PID=$!
"$WAITFOR" mount "$MNT"

ls "$MNT"
[ $(ls $MNT) ] && fail nonempty1
[ $(ls $MNT | wc -l) -eq 0 ] || fail nonempty2

echo 47 >"$MNT"/favorite_number
mkdir "$MNT"/likes
echo true  >"$MNT"/likes/dogs
echo false >"$MNT"/likes/cats
touch "$MNT"/mistakes
echo Michael Greenberg >"$MNT"/name
echo https://mgree.github.io >"$MNT"/website

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

cat "$FILE"
diff "$FILE" "$EXP" || fail diff

rm "$FILE" "$EXP"
rmdir "$MNT"
