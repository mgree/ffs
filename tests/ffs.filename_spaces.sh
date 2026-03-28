#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

printf -- "---\nfield one: 1\nfield two: 2\nfield three: 3" >"$EXP"

ffs -m "$MNT" --target yaml -o "$OUT" --munge filter ../yaml/spaces.yaml &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (field\ one*field\ two) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/field\ one)" -eq 1 ] || fail one
[ "$(cat $MNT/field\ two)" -eq 2 ] || fail two
echo 3 >"$MNT"/field\ three

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

grep "field three: 3" $OUT >/dev/null 2>&1 || fail three

sort $OUT >$OUT.yaml
sort $EXP >$EXP.yaml
diff $OUT.yaml $EXP.yaml || fail diff

rmdir "$MNT" || fail mount
rm "$OUT" "$EXP"
