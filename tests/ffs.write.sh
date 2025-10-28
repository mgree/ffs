#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm -rf "$EXP"
    fi
    exit 1
}

MNT=$(mktemp -d)
EXP=$(mktemp -d)

cat >"${EXP}/4" <<EOF
hi
hello
EOF

ffs -m "$MNT" ../json/list.json &
PID=$!
sleep 2
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
umount "$MNT" || fail unmount
sleep 1

kill -0 $PID >/dev/null 2>&1 && fail process

rmdir "$MNT" || fail mount
rm -rf "$EXP"

