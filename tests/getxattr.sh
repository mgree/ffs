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
    getattr() {
        attr=$1
        shift
        getfattr --name="$attr" "$@"
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

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/object.json &
PID=$!
sleep 2
case $(ls "$MNT") in
    (eyes*fingernails*human*name) ;;
    (*) fail ls;;
esac

[ "$(getattr user.type $MNT)"             = "named"   ] || fail root
[ "$(getattr user.type $MNT/name)"        = "string"  ] || fail name
[ "$(getattr user.type $MNT/eyes)"        = "float"   ] || fail eyes
[ "$(getattr user.type $MNT/fingernails)" = "float"   ] || fail fingernails
[ "$(getattr user.type $MNT/human)"       = "boolean" ] || fail human

umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
