#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$OUT" "$EXP"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

printf -- "---\nfield one: 1\nfield two: 2\nfield three: 3" >"$EXP"

ffs -m "$MNT" --target yaml -o "$OUT" --munge filter ../yaml/spaces.yaml &
PID=$!
sleep 2
case $(ls "$MNT") in
    (field\ one*field\ two) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/field\ one)" -eq 1 ] || fail one
[ "$(cat $MNT/field\ two)" -eq 2 ] || fail two
echo 3 >"$MNT"/field\ three

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

grep "field three: 3" $OUT >/dev/null 2>&1 || fail three

sort $OUT >$OUT.yaml
sort $EXP >$EXP.yaml
diff $OUT.yaml $EXP.yaml || fail diff

rmdir "$MNT" || fail mount
rm "$OUT" "$EXP"
