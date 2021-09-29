#!/bin/sh

set -e

TIMESTAMP=$(date +"%Y%m%d_%H:%M:%S")

NUM_RUNS_DEFAULT=10
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

mkdir ${TIMESTAMP}

./mk_micro.sh
MICRO_RAW=$(mktemp)

printf "BENCHMARKING LAZY MODE\n"

BENCH_LAZY="${TIMESTAMP}/lazy_bench.log"
./bench.sh $ARGS >"$BENCH_LAZY"

./bench.sh -d micro $ARGS >"$MICRO_RAW"
MICRO_LAZY="${TIMESTAMP}/lazy_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO_LAZY"

printf "BENCHMARKING FORCE MODE\n"

BENCH_FORCE="${TIMESTAMP}/force_bench.log"
FFS_ARGS="--force-early" ./bench.sh $ARGS >"$BENCH_FORCE"

FFS_ARGS="--force-early" ./bench.sh -d micro $ARGS >"$MICRO_RAW"
MICRO_FORCE="${TIMESTAMP}/force_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO_FORCE"

printf "BENCHMARKING EAGER MODE\n"

BENCH_EAGER="${TIMESTAMP}/eager_bench.log"
FFS_ARGS="--eager" ./bench.sh $ARGS >"$BENCH_EAGER"

FFS_ARGS="--eager" ./bench.sh -d micro $ARGS >"$MICRO_RAW"
MICRO_EAGER="${TIMESTAMP}/eager_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO_EAGER"

rm "$MICRO_RAW"

./generate_charts.R "$BENCH_LAZY"  "$MICRO_LAZY"
./generate_charts.R "$BENCH_FORCE"  "$MICRO_FORCE"
./generate_charts.R "$BENCH_EAGER" "$MICRO_EAGER"
