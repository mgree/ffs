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

unpack --into "$MNT" ../json/nlink.json || fail unpack

cd "$MNT"
case $(ls) in
    (child1*child2*child3) ;;
    (*) fail ls;;
esac
[ -d . ] && [ -d child1 ] && [ -f child2 ] && [ -d child3 ] || fail filetypes
# APFS on macOS counts directories differently
if [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]
then
    MACOS_DIR=1
else
    MACOS_DIR=0
fi
[ $(num_links      .) -eq $((4 + MACOS_DIR)) ] || fail root   # parent + self + child1 + child3
[ $(num_links child1) -eq $((2 + MACOS_DIR)) ] || fail child1 # parent + self
[ $(num_links child2) -eq 1 ]                  || fail child2 # parent
[ $(num_links child3) -eq $((2 + MACOS_DIR)) ] || fail child3 # parent + self
cd - >/dev/null 2>&1

pack "$MNT" || fail pack
rm -r "$MNT" || fail mount
