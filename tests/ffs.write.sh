#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
EXP=$(mktemp -d)

testcase_cleanup() { rm -rf "$EXP"; }

cat >"${EXP}/4" <<EOF
hi
hello
EOF

ffs -m "$MNT" ../json/list.json &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"
case $(ls) in
    (0*1*2*3) ;;
    (*) fail ls;;
esac
echo hi >4
[ $(cat 4) = "hi" ] || fail write1
echo hello >>4
diff 4 "${EXP}/4" || fail write2
cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
rm -rf "$EXP"
