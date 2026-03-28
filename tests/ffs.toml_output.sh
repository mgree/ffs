#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$TOML"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
TOML=$(mktemp).toml

cp ../toml/eg.toml "$TOML"

ffs -i -m "$MNT" "$TOML" &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (clients*database*owner*servers*title) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/title)" = "TOML Example" ] || fail title
[ "$(cat $MNT/owner/dob)" = "1979-05-27T07:32:00-08:00" ] || fail dob
echo aleph >"$MNT/clients/hosts/2"
echo tav >"$MNT/clients/hosts/3"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

ffs --readonly --no-output -m "$MNT" "$TOML" &
PID=$!
"$WAITFOR" mount "$MNT"
[ "$(cat $MNT/clients/hosts/0)" = "alpha" ] || fail hosts0
[ "$(cat $MNT/clients/hosts/1)" = "omega" ] || fail hosts1
[ "$(cat $MNT/clients/hosts/2)" = "aleph" ] || fail hosts2
[ "$(cat $MNT/clients/hosts/3)" = "tav"   ] || fail hosts3
"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2

rmdir "$MNT" || fail mount
rm "$TOML"
