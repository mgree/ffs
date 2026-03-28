#!/bin/sh

if ! [ "$RUNNER_OS" = "macOS" ] && ! [ "$(uname)" = "Darwin" ]
then
    echo "This test only runs under macOS; you're using ${RUNNER_OS-$(uname)}" >&2
    exit 0
fi

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

listattr() {
    xattr -l "$@"
}
getattr() {
    attr=$1
    shift
    xattr -p "$attr" "$@"
}
setattr() {
    attr="$1"
    val="$2"
    shift 2
    xattr -w "$attr" "$val" "$@"
}
rmattr() {
    attr=$1
    shift
    xattr -d "$attr" "$@"
}

typeof() {
    getattr user.type "$@"
}

MNT=$(mktemp -d)
OUT=$(mktemp)

ffs -m "$MNT" --no-xattr -o $OUT --target json ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

[ "$(typeof $MNT)"             = "named"   ] && fail root
[ "$(typeof $MNT/name)"        = "string"  ] && fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] && fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] && fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] && fail human

setattr user.type list "$MNT" || fail set1

[ "$(typeof $MNT)" = "list"   ] || fail "macos override"

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process1

grep -e '"\._."' "$OUT" >/dev/null 2>&1 && fail metadata1

# now try to keep the metadata
ffs -m "$MNT" --no-xattr --keep-macos-xattr -o $OUT --target json ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

setattr user.type list "$MNT"

"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2

grep -e '"\._."' "$OUT" >/dev/null 2>&1 || fail metadata2

# now try to keep the metadata but also have the FS store it
ffs -m "$MNT" --keep-macos-xattr -o $OUT --target json ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

setattr user.type list "$MNT"

"$WAITFOR" umount "$MNT" || fail unmount3
"$WAITFOR" exit $PID || fail process3

grep -e '"\._."' "$OUT" >/dev/null 2>&1 && fail metadata3

rmdir "$MNT" || fail mount
rm "$OUT"
