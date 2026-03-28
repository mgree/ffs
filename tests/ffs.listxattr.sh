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
    which getfattr || fail getfattr
    listattr() {
        getfattr --match=- "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    listattr() {
        xattr -l "$@"
    }
else
    fail os
fi

listattr_ok() {
    listattr $1 | grep "user.type"
}

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"

listattr_ok "$MNT" || fail root
listattr_ok "$MNT"/name || fail name
listattr_ok "$MNT"/eyes || fail eyes
listattr_ok "$MNT"/fingernails || fail fingernails
listattr_ok "$MNT"/human || fail human

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
