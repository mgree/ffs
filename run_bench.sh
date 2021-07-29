#!/bin/sh

set -e

TIMESTAMP=$(date +"%Y%m%d_%H:%M:%S")

usage() {
    exec >&2
    printf "Usage: %s [-n NUM_RUNS]\n\n" "$(basename $0)"
    printf "       -n NUM_RUNS    the number of runs for each test case (defaults to $NUM_RUNS_DEFAULT)\n"
    exit 2
}

ARGS=""
while getopts ":n:h" opt
do
    case "$opt" in
        (n) if [ $((OPTARG)) -le 0 ]
            then
                printf "NUM_RUNS must be a positive number; got '%s'\n\n" "$OPTARG"
                usage
            fi
            ARGS="$ARGS -n $OPTARG"
            ;;
        (h) usage
            ;;
        (*) printf "Unrecognized argument '%s'\n\n" "$OPTARG"
            usage
            ;;
    esac
done
shift $((OPTIND - 1))
[ $# -eq 0 ] || usage

cd bench

BENCH="../${TIMESTAMP}_bench.log"
./bench.sh $ARGS >"$BENCH"

./mk_micro.sh
MICRO_RAW=$(mktemp)
./bench.sh -d micro $ARGS >"$MICRO_RAW"
MICRO="../${TIMESTAMP}_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO"
rm "$MICRO_RAW"

./generate_charts.R "$BENCH" "$MICRO"
