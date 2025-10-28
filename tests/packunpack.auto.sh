#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm "$FILE" "$EXP"
        rm -r "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)
FILE=$(mktemp).json

echo '{}' >"$FILE"

EXP=$(mktemp)

printf '{"favorite_number":47,"likes":{"cats":false,"dogs":true},"mistakes":null,"name":"Michael Greenberg","website":"https://mgree.github.io"}' >"$EXP"

unpack "$FILE" --into "$MNT" || fail unpack

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

pack "$MNT" -o "$FILE" || fail pack

cat "$FILE"
diff "$FILE" "$EXP" || fail diff

rm "$FILE" "$EXP"
rm -r "$MNT"
