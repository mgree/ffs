#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rmdir "$MNT"
        rm "$OUT"
    fi
    exit 1
}

WAITFOR="$(cd ../utils; pwd)/waitfor"

MNT=$(mktemp -d)
OUT=$(mktemp)

ffs -m "$MNT" --target toml -o "$OUT" --pretty ../toml/single.toml &
PID=$!
"$WAITFOR" mount "$MNT"

cat >"$MNT"/info <<EOF
Duncan MacLeod
as played by
Adrian Paul
EOF

"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID || fail process

[ "$(cat $OUT | wc -l)" -eq 5 ] || fail lines
[ "$(head -n 1 $OUT)" = 'info = """' ] || fail multi

rmdir "$MNT" || fail mount
rm "$OUT"
