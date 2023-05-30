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

printf -- "---\nfield one: 1\nfield two: 2\nfield three: 3" >"$EXP"

unpack --into "$MNT" --munge filter ../yaml/spaces.yaml

case $(ls "$MNT") in
    (field\ one*field\ two) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/field\ one)" -eq 1 ] || fail one
[ "$(cat $MNT/field\ two)" -eq 2 ] || fail two
echo 3 >"$MNT"/field\ three

pack --target yaml -o "$OUT" --munge filter "$MNT"

grep "field three: 3" $OUT >/dev/null 2>&1 || fail three

sort $OUT >$OUT.yaml
sort $EXP >$EXP.yaml
diff $OUT.yaml $EXP.yaml || fail diff

rm -r "$MNT" || fail mount
rm "$OUT" "$EXP"
