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

unpack --into "$MNT" ../json/object.json

rmattr user.type $MNT || fail "root user.type"
rmattr user.fake $MNT && fail "root user.fake"
rmattr user.type "$MNT/name" || fail " user.type"

pack "$MNT"
:
rm -r "$MNT" || fail mount
