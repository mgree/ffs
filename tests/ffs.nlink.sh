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
    num_links() {
        stat --format %h "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    num_links() {
        stat -f %l "$@"
    }
else
    fail os
fi

MNT=$(mktemp -d)

ffs -m "$MNT" ../json/nlink.json &
PID=$!
sleep 2
cd "$MNT"
case $(ls) in
    (child1*child2*child3) ;;
    (*) fail ls;;
esac
[ -d . ] && [ -d child1 ] && [ -f child2 ] && [ -d child3 ] || fail filetypes
[ $(num_links      .) -eq 4 ] || fail root   # parent + self + child1 + child3
[ $(num_links child1) -eq 2 ] || fail child1 # parent + self
[ $(num_links child2) -eq 1 ] || fail child2 # parent
[ $(num_links child3) -eq 2 ] || fail child3 # parent + self
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
