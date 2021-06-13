#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
    fi
    exit 1
}

MNT=$(mktemp -d)

ffs --unpadded "$MNT" ../json/list2.json &
PID=$!
sleep 1
cd "$MNT"
case $(ls) in
    (0*1*10*2*3*4*5*6*7*8*9) ;;
    (*) fail ls;;
esac
[ "$(cat  0)" -eq  0 ] || fail  0
[ "$(cat  1)" -eq  1 ] || fail  1 
[ "$(cat  2)" -eq  2 ] || fail  2 
[ "$(cat  3)" -eq  3 ] || fail  3 
[ "$(cat  4)" -eq  4 ] || fail  4 
[ "$(cat  5)" -eq  5 ] || fail  5 
[ "$(cat  6)" -eq  6 ] || fail  6 
[ "$(cat  7)" -eq  7 ] || fail  7 
[ "$(cat  8)" -eq  8 ] || fail  8 
[ "$(cat  9)" -eq  9 ] || fail  9 
[ "$(cat 10)" -eq 10 ] || fail 10 
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
