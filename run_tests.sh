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
if ! which unpack >/dev/null 2>&1
then
    DEBUG="$(pwd)/target/debug"
    [ -x "$DEBUG/unpack" ] || {
        echo Couldn\'t find unpack on "$PATH" or in "$DEBUG". >&2
        echo Are you in the root directory of the repo? >&2
        exit 1
    }
    PATH="$DEBUG:$PATH"
fi
if ! which pack >/dev/null 2>&1
then
    DEBUG="$(pwd)/target/debug"
    [ -x "$DEBUG/pack" ] || {
        echo Couldn\'t find pack on "$PATH" or in "$DEBUG". >&2
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
TESTS="$(find . -name "$1*.sh")"

# spawn 'em all in parallel
for test in $TESTS
do
    tname="$(basename ${test%*.sh})"
    printf "========== STARTING TEST: $tname\n"
    (RUST_LOG="ffs=debug,unpack=debug,pack=debug,fuser=debug"; export RUST_LOG; ./${test} >$LOG/$tname.out 2>$LOG/$tname.err; echo $?>$LOG/$tname.ec) &
    : $((TOTAL += 1))

    # don't slam 'em
    if [ $((TOTAL % 4)) -eq 0 ]
    then
        wait %-
    fi
done

wait

for test in $TESTS
do
    tname="$(basename ${test%*.sh})"
    if [ "$(cat $LOG/$tname.ec)" -eq 0 ]
    then
        printf "========== PASSED: $tname\n"
    else
        printf "========== FAILED: $tname (ec=$(cat $LOG/$tname.ec))\n"
        : $((FAILED += 1))
    fi

    # just always capture output in the CI logs
    if [ "$(cat $LOG/$tname.ec)" -ne 0 ] || [ "$CI" ]
    then
        printf "<<<<<<<<<< STDOUT\n"
        cat $LOG/$tname.out
        printf "<<<<<<<<<< STDERR\n"
        cat $LOG/$tname.err
        printf "\n"
    fi
done

printf "$((TOTAL - FAILED))/$((TOTAL)) tests passed\n"

rm -r $LOG
[ $FAILED -eq 0 ] || exit 1
