#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$TOML"
    fi
    exit 1
}

MNT=$(mktemp -d)
TOML=$(mktemp)

mv "$TOML" "$TOML".toml
TOML="$TOML".toml

cp ../toml/eg.toml "$TOML"

unpack --into "$MNT" "$TOML"

case $(ls "$MNT") in
    (clients*database*owner*servers*title) ;;
    (*) fail ls;;
esac
[ "$(cat $MNT/title)" = "TOML Example" ] || fail title
[ "$(cat $MNT/owner/dob)" = "1979-05-27T07:32:00-08:00" ] || fail dob
echo aleph >"$MNT/clients/hosts/2"
echo tav >"$MNT/clients/hosts/3"
pack -o "$TOML" "$MNT"
rm -r "$MNT"

unpack --into "$MNT" "$TOML"

[ "$(cat $MNT/clients/hosts/0)" = "alpha" ] || fail hosts0
[ "$(cat $MNT/clients/hosts/1)" = "omega" ] || fail hosts1
[ "$(cat $MNT/clients/hosts/2)" = "aleph" ] || fail hosts2
[ "$(cat $MNT/clients/hosts/3)" = "tav"   ] || fail hosts3

rm -r "$MNT" || fail mount
rm "$TOML"
