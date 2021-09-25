#!/bin/sh

if ! which ffs >/dev/null 2>&1
then
    DEBUG="$(pwd)/target/debug"
    [ -x "$DEBUG/ffs" ] || {
        echo Couldn\'t find ffs on "$PATH" or in "$DEBUG". >&2
        echo Are you in the root directory of the repo? >&2
        exit 1
    }
    PATH="$DEBUG:$PATH"
fi

TOTAL=0
FAILED=0
ERRORS=""
cd tests

LOG=$(mktemp -d)

# spawn 'em all in parallel
for test in *.sh
do
    tname="$(basename ${test%*.sh})"
    printf "========== STARTING TEST: $tname\n"
    (RUST_LOG="ffs=debug,fuser=debug"; export RUST_LOG; ./${test} >$LOG/$tname.out 2>$LOG/$tname.err; echo $?>$LOG/$tname.ec) &
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
