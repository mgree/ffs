#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs "$MNT" ../json/list2.json &
PID=$!
sleep 1
cd "$MNT"
case $(ls) in
    (00*01*02*03*04*05*06*07*08*09*10) ;;
    (*) fail ls;;
esac
[ "$(cat 00)" -eq  0 ] || fail  0
[ "$(cat 01)" -eq  1 ] || fail  1 
[ "$(cat 02)" -eq  2 ] || fail  2 
[ "$(cat 03)" -eq  3 ] || fail  3 
[ "$(cat 04)" -eq  4 ] || fail  4 
[ "$(cat 05)" -eq  5 ] || fail  5 
[ "$(cat 06)" -eq  6 ] || fail  6 
[ "$(cat 07)" -eq  7 ] || fail  7 
[ "$(cat 08)" -eq  8 ] || fail  8 
[ "$(cat 09)" -eq  9 ] || fail  9 
[ "$(cat 10)" -eq 10 ] || fail 10 
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
