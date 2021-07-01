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
    getattr() {
        attr=$1
        shift
        echo getfattr -n "$attr" "$@"        
        getfattr -n "$attr" "$@"
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
sleep 2
case $(ls "$MNT") in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac

[ "$(typeof $MNT)"             = "named"   ] || { echo root didn\'t work; typeof $MNT; }
[ "$(typeof $MNT/name)"        = "string"  ] || fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] || fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] || fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] || fail human

umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
