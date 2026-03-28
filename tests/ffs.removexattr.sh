#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    which setfattr || fail setfattr
    rmattr() {
        attr=$1
        shift
        setfattr -x "$attr" "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    rmattr() {
        attr=$1
        shift
        xattr -d "$attr" "$@"
    }
else
    fail os
fi

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

rmattr user.type $MNT && fail "root user.type"
rmattr user.fake $MNT && fail "root user.fake"
rmattr user.type "$MNT/name" && fail "root user.type"

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
