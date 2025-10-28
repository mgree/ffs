#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$MNT"
        rmdir "$MNT"
        rm "$OUT"
    fi
    exit 1
}

MNT=$(mktemp -d)
OUT=$(mktemp)

ffs -m "$MNT" --target toml -o "$OUT" --pretty ../toml/single.toml &
PID=$!
sleep 2

cat >"$MNT"/info <<EOF
Duncan MacLeod
as played by
Adrian Paul
EOF

umount "$MNT" || fail unmount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

[ "$(cat $OUT | wc -l)" -eq 5 ] || fail lines
[ "$(head -n 1 $OUT)" = 'info = """' ] || fail multi

rmdir "$MNT" || fail mount
rm "$OUT"
