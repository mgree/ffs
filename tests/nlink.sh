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

MNT=$(mktemp -d)

ffs "$MNT" ../json/nlink.json &
PID=$!
sleep 1
cd "$MNT"
case $(ls) in
    (child1*child2*child3) ;;
    (*) fail ls;;
esac
[ -d . ] && [ -d child1 ] && [ -f child2 ] && [ -d child3 ] || fail filetypes
[ $(stat -r .      | cut -d' ' -f 4) -eq 4 ] || fail root   # parent + self + child1 + child3
[ $(stat -r child1 | cut -d' ' -f 4) -eq 2 ] || fail child1 # parent + self
[ $(stat -r child2 | cut -d' ' -f 4) -eq 1 ] || fail child2 # parent
[ $(stat -r child3 | cut -d' ' -f 4) -eq 2 ] || fail child3 # parent + self
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
