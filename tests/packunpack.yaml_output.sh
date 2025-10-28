#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$YAML"
    fi
    exit 1
}

MNT=$(mktemp -d)
YAML=$(mktemp)

mv "$YAML" "$YAML".yaml
YAML="$YAML".yaml

cp ../yaml/invoice.yaml "$YAML"

unpack --into "$MNT" "$YAML" || fail unpack1
case $(ls "$MNT") in
    (bill-to*comments*date*invoice*product*ship-to*tax*total) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/date)" = "2001-01-23" ] || fail date
[ "$(cat $MNT/product/0/description)" = "Basketball" ] || fail product
echo orange >"$MNT/product/0/color"
echo pink >"$MNT/product/1/color"
pack -o "$YAML" "$MNT" || fail pack1
rm -r "$MNT"

unpack --into "$MNT" "$YAML" || fail unpack2
[ "$(cat $MNT/product/0/description)" = "Basketball" ] || fail desc1
[ "$(cat $MNT/product/0/color)"       = "orange" ]     || fail color1
[ "$(cat $MNT/product/1/description)" = "Super Hoop" ] || fail desc2
[ "$(cat $MNT/product/1/color)"       = "pink"   ]     || fail color2

pack "$MNT" || fail pack2
rm -r "$MNT" || fail mount
rm "$YAML"
