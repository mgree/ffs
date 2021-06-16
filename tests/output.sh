#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$TGT"
        rm "$TGT2"
    fi
    exit 1
}

MNT=$(mktemp -d)
TGT=$(mktemp)
TGT2=$(mktemp)

ffs "$MNT" ../json/object.json >"$TGT" &
PID=$!
sleep 2
mkdir "$MNT"/pockets
echo keys >"$MNT"/pockets/pants
echo pen >"$MNT"/pockets/shirt
cd - >/dev/null 2>&1
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

# easiest to just test using ffs, but would be cool to get outside validation
[ -f "$TGT" ] || fail output1
if [ "$RUNNER_OS" = "Linux" ]
then
    echo ABORTING TEST, currently broken on Linux (see https://github.com/cberner/fuser/issues/153)
    exit 0
fi
[ -s "$TGT" ] || fail output2
cat "$TGT"
stat "$TGT"
ffs --no-output "$MNT" "$TGT" >"$TGT2" &
PID=$!
sleep 2

case $(ls "$MNT") in
    (eyes*fingernails*human*name*pockets) ;;
    (*) fail ls1;;
esac
case $(ls "$MNT"/pockets) in
    (pants*shirt) ;;
    (*) fail ls2;;
esac

[ "$(cat $MNT/name)" = "Michael Greenberg" ] || fail name
[ "$(cat $MNT/eyes)" -eq 2 ] || fail eyes
[ "$(cat $MNT/fingernails)" -eq 10 ] || fail fingernails
[ "$(cat $MNT/human)" = "true" ] || fail human
[ "$(cat $MNT/pockets/pants)" = "keys" ] || fail pants
[ "$(cat $MNT/pockets/shirt)" = "pen" ] || fail shirt

umount "$MNT" || fail unmount2
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process2


rmdir "$MNT" || fail mount
rm "$TGT"
rm "$TGT2"
