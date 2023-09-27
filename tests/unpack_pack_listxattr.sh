#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
    fi
    exit 1
}

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

unpack --into "$MNT" ../json/object.json || fail unpack

listattr_ok "$MNT" || fail root
listattr_ok "$MNT"/name || fail name
listattr_ok "$MNT"/eyes || fail eyes
listattr_ok "$MNT"/fingernails || fail fingernails
listattr_ok "$MNT"/human || fail human

pack "$MNT" || fail pack
rm -r "$MNT" || fail mount
