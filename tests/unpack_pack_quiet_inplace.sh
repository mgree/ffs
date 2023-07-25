#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
        rm "$JSON" "$LOG"
    fi
    exit 1
}

MNT=$(mktemp -d)
JSON=$(mktemp)
LOG=$(mktemp)

cp ../json/object.json "$JSON"

unpack -q --into "$MNT" "$JSON" >>"$LOG" 2>&1 || fail unpack1

echo hi >"$MNT"/greeting

pack -q -o "$JSON" "$MNT" >>"$LOG" 2>&1 || fail pack1
diff ../json/object.json "$JSON" >/dev/null && fail same
[ "$(cat $LOG)" = "" ] || fail quiet
rm -r "$MNT"

unpack --into "$MNT" "$JSON" || fail unpack2

[ "$(cat $MNT/greeting)" = "hi" ] || fail updated

pack "$MNT" || fail pack2

rm -r "$MNT" || fail mount
rm "$JSON" || fail copy
