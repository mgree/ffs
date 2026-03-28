#!/bin/sh
#
# from https://github.com/mgree/ffs/issues/42

fail() {
    echo FAILED: $1
    if [ mountpoint -q "$MNT" ]
    then
        "$WAITFOR" umount "$MNT"
        rm -r "$D"
    fi
    if [ "$PID" ]
    then
       kill -KILL "$PID"
    fi
    exit 1
}

TESTS="$(pwd)"
TIMEOUT="$(cd ../utils; pwd)/timeout"
WAITFOR="$(cd ../utils; pwd)/waitfor"

D=$(mktemp -d)

cp ../json/single.json "$D"/single.json

cd "$D"
ffs -i single.json &
PID=$!
"$WAITFOR" mount single
MNT="$D"/single
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

"$TIMEOUT" -t 3 -l single.timeout ffs -i single.json 2>single.err
NESTEDSTATUS=$?
[ -f single.timeout ] && fail timeout
[ -s single.err ] || fail error
rm single.err
[ $NESTEDSTATUS -eq 1 ] || fail status

case $(ls) in
    (onlyone*single.json) ;;
    (*) fail ls3;;
esac

cd "$D"
case $(ls) in
    (single*single.json) ;;
    (*) fail ls4;;
esac
"$WAITFOR" umount single || fail umount
"$WAITFOR" exit $PID || fail process

cd "$TESTS"
rm -r "$D" || fail mount
