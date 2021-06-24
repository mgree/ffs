#!/bin/sh

PATH="$(pwd)/target/debug:$PATH"

TOTAL=0
FAILED=0
ERRORS=""
cd tests

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$ERR"
    fi
    exit 1
}

ERR=$(mktemp)
MNT=$(mktemp -d)
RUST_LOG="ffs=debug" ffs -d "$MNT" ../json/object.json &
PID=$!
sleep 2
chown -v :nobody "$MNT"/name 2>$ERR >&2 && fail "chgrp1: $(cat $ERR)"
[ -s "$ERR" ] || fail "chgrp1 error: $(cat $ERR)"
groups
ls -l "$MNT"/name
echo $(groups | cut -d' ' -f 1)
chown -v :$(groups | cut -d' ' -f 1) "$MNT"/name 2>$ERR >&2 || echo "chgrp2: $(cat $ERR)"
chown -v $(whoami) "$MNT"/name 2>$ERR >&2 || echo "chown: $(cat $ERR)"
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1
rmdir "$MNT"
rm "$ERR"


LOG=$(mktemp -d)

# spawn 'em all in parallel
for test in *.sh
do
    tname="$(basename ${test%*.sh})"
    printf "========== STARTING TEST: $tname\n"
    (RUST_LOG="ffs=debug"; export RUST_LOG; ./${test} >$LOG/$tname.out 2>$LOG/$tname.nerr; echo $?>$LOG/$tname.ec) &
    : $((TOTAL += 1))

    # don't slam 'em
    if [ $((TOTAL % 4)) -eq 0 ]
    then
        wait %-
    fi
done

wait

for test in *.sh
do
    tname="$(basename ${test%*.sh})"
    if [ "$(cat $LOG/$tname.ec)" -eq 0 ]
    then
        printf "========== PASSED: $tname\n"
    else
        printf "========== FAILED: $tname (ec=$(cat $LOG/$tname.ec))\n"
        printf "<<<<<<<<<< STDOUT\n"
        cat $LOG/$tname.out
        printf "<<<<<<<<<< STDERR\n"
        cat $LOG/$tname.err
        printf "\n"
        : $((FAILED += 1))
    fi
done

printf "$((TOTAL - FAILED))/$((TOTAL)) tests passed\n"

rm -r $LOG
[ $FAILED -eq 0 ] || exit 1
