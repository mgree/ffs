#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

unpack --into "$MNT" ../toml/single.toml

cat >"$MNT"/info <<EOF
Duncan MacLeod
as played by
Adrian Paul
EOF

pack --target toml -o "$OUT" --pretty "$MNT"

[ "$(cat $OUT | wc -l)" -eq 5 ] || fail lines
[ "$(head -n 1 $OUT)" = "info = '''" ] || fail multi

rm -r "$MNT" || fail mount
rm "$OUT"
