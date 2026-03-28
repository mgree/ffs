#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    which getfattr || fail getfattr
    getattr() {
        attr=$1
        shift
        getfattr -n "$attr" --only-values "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    getattr() {
        attr=$1
        shift
        xattr -p "$attr" "$@"
    }
else
    fail os
fi

typeof() {
    getattr user.type "$@"
}

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

[ "$(typeof $MNT)"             = "named"   ] || fail root
[ "$(typeof $MNT/name)"        = "string"  ] || fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] || fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] || fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] || fail human

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
