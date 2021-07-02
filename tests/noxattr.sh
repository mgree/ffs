#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    which getfattr || fail getfattr
    which setfattr || fail setfattr
    getattr() {
        attr=$1
        shift
        getfattr -n "$attr" --only-values "$@"
    }
    setattr() {
        attr="$1"
        val="$2"
        shift 2
        setfattr -n "$attr" -v "$val" "$@"
    }
    listattr() {
        getfattr --match=- "$@"
    }
    rmattr() {
        attr=$1
        shift
        setfattr -x "$attr" "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
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

else
    fail os
fi

typeof() {
    getattr user.type "$@"
}

MNT=$(mktemp -d)

ffs -m "$MNT" --no-xattr ../json/object.json &
PID=$!
sleep 2

[ "$(typeof $MNT)"             = "named"   ] && fail root
[ "$(typeof $MNT/name)"        = "string"  ] && fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] && fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] && fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] && fail human


if ! [ "$RUNNER_OS" = "macOS" ] && ! [ "$(uname)" = "Darwin" ]
then
    # some version of macos will just store these in ._* files if the
    # FS refuses them
    #
    # best to just not test it for now :(
    setattr user.type list $MNT && fail "root user.type"
    setattr user.fake list $MNT && fail "root user.fake"
fi

listattr_fails() {
    ! listattr $1 | grep "user.type"
}

listattr_fails "$MNT" || fail root
listattr_fails "$MNT"/name || fail name
listattr_fails "$MNT"/eyes || fail eyes
listattr_fails "$MNT"/fingernails || fail fingernails
listattr_fails "$MNT"/human || fail human

rmattr user.type $MNT && fail "root user.type"
rmattr user.fake $MNT && fail "root user.fake"
rmattr user.type "$MNT/name" && fail "root user.type"

umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
