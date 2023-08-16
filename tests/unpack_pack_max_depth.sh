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
MNT2=$(mktemp -d)
OUT=$(mktemp)
mv "$OUT" "$OUT".json
OUT="$OUT".json

mkdir -p "$MNT"/1/2/3/4/5/6/7/8/9/10
echo "file" >"$MNT"/1/2/3/4/5/6/7/8/9/10/file
mkdir -p "$MNT2"/symlink/test
cd "$MNT2"/symlink/test
ln -s "$MNT" link

EXP=$(mktemp)

printf '{"1":{"2":{"3":{"4":{"5":{"6":{}}}}}}}' >"$EXP"
pack --max-depth 6 -o "$OUT" -- "$MNT" || fail pack
diff "$OUT" "$EXP" || fail diff

printf '{"1":{"2":{"3":{"4":{"5":{"6":{"7":{"8":{"9":{"10":{}}}}}}}}}}}' >"$EXP"
pack --max-depth 10 -o "$OUT" -- "$MNT" || fail pack2
diff "$OUT" "$EXP" || fail diff2

printf '{"1":{"2":{"3":{"4":{"5":{"6":{"7":{"8":{"9":{"10":{"file":"file"}}}}}}}}}}}' >"$EXP"
pack --max-depth 11 -o "$OUT" -- "$MNT" || fail pack3
diff "$OUT" "$EXP" || fail diff3

printf '{"symlink":{"test":{"link":{"1":{"2":{"3":{"4":{"5":{"6":{"7":{"8":{"9":{"10":{"file":"file"}}}}}}}}}}}}}}' >"$EXP"
pack --max-depth 14 -o "$OUT" -L --allow-symlink-escape -- "$MNT2" || fail pack4
diff "$OUT" "$EXP" || fail diff4

rm "$OUT" "$EXP"
rm -r "$MNT"
