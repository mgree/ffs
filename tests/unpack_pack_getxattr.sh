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

unpack --into "$MNT" ../json/object.json

[ "$(typeof $MNT)"             = "named"   ] || fail root
[ "$(typeof $MNT/name)"        = "string"  ] || fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] || fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] || fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] || fail human

rm -r "$MNT" || fail mount
