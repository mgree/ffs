#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$YAML"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
YAML=$(mktemp).yaml

cp ../yaml/invoice.yaml "$YAML"

ffs -i -m "$MNT" "$YAML" &
PID=$!
"$WAITFOR" mount "$MNT"
case $(ls "$MNT") in
    (bill-to*comments*date*invoice*product*ship-to*tax*total) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/date)" = "2001-01-23" ] || fail date
[ "$(cat $MNT/product/0/description)" = "Basketball" ] || fail product
echo orange >"$MNT/product/0/color"
echo pink >"$MNT/product/1/color"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

ffs --readonly --no-output -m "$MNT" "$YAML" &
PID=$!
"$WAITFOR" mount "$MNT"
[ "$(cat $MNT/product/0/description)" = "Basketball" ] || fail desc1
[ "$(cat $MNT/product/0/color)"       = "orange" ]     || fail color1
[ "$(cat $MNT/product/1/description)" = "Super Hoop" ] || fail desc2
[ "$(cat $MNT/product/1/color)"       = "pink"   ]     || fail
"$WAITFOR" umount "$MNT" || fail unmount2
"$WAITFOR" exit $PID || fail process2

rmdir "$MNT" || fail mount
rm "$YAML"
