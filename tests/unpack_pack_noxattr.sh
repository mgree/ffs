#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
    fi
    if [ "$MNT2" ]
    then
        rm -r "$MNT2"
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

unpack --into "$MNT" --no-xattr ../json/object.json

[ "$(typeof $MNT)"             = "named"   ] && fail root
[ "$(typeof $MNT/name)"        = "string"  ] && fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] && fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] && fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] && fail human



listattr_fails() {
    ! listattr $1 | grep "user.type"
}

listattr_fails "$MNT" || fail root
listattr_fails "$MNT"/name || fail name
listattr_fails "$MNT"/eyes || fail eyes
listattr_fails "$MNT"/fingernails || fail fingernails
listattr_fails "$MNT"/human || fail human

# unlike ffs, we can set xattrs even if unpack didn't
setattr user.type list "$MNT" || fail "root user.type"
setattr user.fake list "$MNT" || fail "root user.fake"

listattr "$MNT" | grep "user.type" || fail "root user.type missing"
listattr "$MNT" | grep "user.fake" || fail "root user.fake missing"

rmattr user.type "$MNT" || fail "root user.type"
rmattr user.fake "$MNT" || fail "root user.fake"
rmattr user.type "$MNT"/name && fail "root user.type"


GOT="$(mktemp)"
pack "$MNT" >"$GOT"
MNT2="$(mktemp -d)"
unpack --into "$MNT2" "$GOT"
diff -r "$MNT" "$MNT2" || fail "modified output"

rm -r "$MNT" || fail mount
rm -r "$MNT2" || fail mount2
