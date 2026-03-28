#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    which setfattr || fail setfattr
    setattr() {
        attr="$1"
        val="$2"
        shift 2
        setfattr -n "$attr" -v "$val" "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    setattr() {
        attr="$1"
        val="$2"
        shift 2
        xattr -w "$attr" "$val" "$@"
    }
else
    fail os
fi

MNT=$(mktemp -d)
OUT=$(mktemp)
EXP=$(mktemp)

testcase_cleanup() { rm -f "$OUT" "$EXP"; }

# NB no newline. this is a little hardcoded for my taste, but yolo
printf '[2,10,"true","Michael Greenberg"]' >"$EXP"

ffs -m "$MNT" --target json -o "$OUT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

setattr user.type list $MNT || fail "root user.type"
setattr user.fake list $MNT && fail "root user.fake"
setattr user.type string "$MNT/human" || fail "human"

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

[ "$(cat $OUT)" = "$(cat $EXP)" ] || fail "different strings"
diff "$OUT" "$EXP" || fail "different files"

rmdir "$MNT" || fail mount
