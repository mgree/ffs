#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

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
"$WAITFOR" mount "$MNT"
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
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
