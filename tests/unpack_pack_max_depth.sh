#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm "$OUT" "$EXP"
        rm -r "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

mkdir -p "$MNT"/1/2/3/4/5/6/7/8/9/10
touch "$MNT"/1/2/3/4/5/6/7/8/9/10/file

EXP=$(mktemp)

printf '{"1":{"2":{"3":{"4":{"5":{"6":{}}}}}}}' >"$EXP"
pack "$MNT" --max-depth 6 -o "$OUT" || fail pack
diff "$OUT" "$EXP" || fail diff

printf '{"1":{"2":{"3":{"4":{"5":{"6":{"7":{"8":{"9":{"10":{}}}}}}}}}}}' >"$EXP"
pack "$MNT" --max-depth 10 -o "$OUT" || fail pack2
diff "$OUT" "$EXP" || fail diff2

rm "$OUT" "$EXP"
rm -r "$MNT"
