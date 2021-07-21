#!/bin/sh
#
# from https://github.com/mgree/ffs/issues/42

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$D"/single
        rm -r "$D"
    fi
    exit 1
}

TESTS="$(pwd)"

D=$(mktemp -d)

cp ../json/single.json "$D"/single.json

cd "$D"
ffs -i single.json &
PID=$!
sleep 2
case $(ls) in
    (single*single.json) ;;
    (*) fail ls1;;
esac

cd single
case $(ls) in
    (onlyone) ;;
    (*) fail ls2;;
esac

cp ../single.json .

"${TESTS}"/timeout -t 3 -l single.timeout ffs -i single.json 2>single.err
NESTEDSTATUS=$?
[ -f single.timeout ] && fail timeout
[ -s single.err ] || fail error
rm single.err
[ $NESTEDSTATUS -eq 2 ] || fail status

case $(ls) in
    (onlyone*single.json) ;;
    (*) fail ls3
esac

cd ..
[ "$D" = "$(PWD)" ] || fail baddir
sleep 1
umount single || fail umount
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process

cd "$TESTS"
rm -r "$D" || fail mount
