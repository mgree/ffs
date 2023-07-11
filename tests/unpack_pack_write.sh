#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
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

unpack --into "$MNT" ../json/list.json
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
pack "$MNT"

rm -r "$MNT" || fail mount
rm -rf "$EXP"

