#!/bin/sh

FFS_TOP=$(realpath "${0%/*}")
DEBUG="$FFS_TOP/target/debug"

detect_tools() {
    if ! which ffs >/dev/null 2>&1
    then
        [ -x "$DEBUG/ffs" ] && PATH="$DEBUG:$PATH"
    fi
    which ffs >/dev/null 2>&1 && HAVE_FFS=1

    if ! which unpack >/dev/null 2>&1
    then
        [ -x "$DEBUG/unpack" ] && PATH="$DEBUG:$PATH"
    fi
    if ! which pack >/dev/null 2>&1
    then
        [ -x "$DEBUG/pack" ] && PATH="$DEBUG:$PATH"
    fi
    which pack unpack >/dev/null 2>&1 && HAVE_PACKUNPACK=1

    [ "$HAVE_FFS" ] || [ "$HAVE_PACK_UNPACK" ]
}

if  ! detect_tools
then
    printf "Couldn't find ffs or pack/unpack; building...\n" >&2
    (cd "$FFS_TOP"; cargo build --workspace)
    if ! detect_tools
    then
        printf "Still couldn't find ffs or pack/unpack after building... giving up!\n" >&2
        exit 2
    fi
fi

[ "$HAVE_FFS" ] || [ "$HAVE_PACKUNPACK" ] || { echo "error: no binaries found; run \`cargo build\` first" >&2; exit 1; }

TOTAL=0
FAILED=0
ERRORS=""
cd tests

LOG=$(mktemp -d)
TESTS="$(find . -name "$1*.sh")"

# spawn 'em all in parallel
for test in $TESTS
do
    script="${test##*/}"
    tname="${script%.sh}"
    tool="${tname%.*}"

    case "$tool" in
        (ffs) [ "$HAVE_FFS" ] || continue;;
        (packunpack) [ "$HAVE_PACKUNPACK" ] || continue;;
    esac

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
    script="${test##*/}"
    tname="${script%.sh}"
    tool="${tname%.*}"

    case "$tool" in
        (ffs) [ "$HAVE_FFS" ] || continue;;
        (packunpack) [ "$HAVE_PACKUNPACK" ] || continue;;
    esac

    if [ "$(cat $LOG/$tname.ec)" -eq 0 ]
    then
        printf "========== PASSED: $tname\n"
    else
        printf "========== FAILED: $tname (ec=$(cat $LOG/$tname.ec))\n"
        : $((FAILED += 1))
        ERRORS="${ERRORS}${ERRORS+ }$tname"
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
printf "FAILING CASES: $ERRORS\n"

rm -r $LOG
[ $FAILED -eq 0 ] || exit 1
