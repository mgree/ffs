#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rm "$FILE" "$EXP"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)
FILE="$MNT"/demo.json

echo '{}' >"$FILE"

EXP=$(mktemp)

printf '{"favorite_number":47,"likes":{"cats":false,"dogs":true},"mistakes":null,"name":"Michael Greenberg","website":"https://mgree.github.io"}' >"$EXP"

ffs  -m "$MNT" -i "$FILE" &
PID=$!
sleep 2
[ $(ls $MNT) ] && fail nonempty

echo 47 >"$MNT"/favorite_number
mkdir "$MNT"/likes
echo true  >"$MNT"/likes/dogs
echo false >"$MNT"/likes/cats
touch "$MNT"/mistakes
echo Michael Greenberg >"$MNT"/name
echo https://mgree.github.io >"$MNT"/website

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

diff "$FILE" "$EXP" || fail diff

rm "$FILE" "$EXP"
rmdir "$MNT"
