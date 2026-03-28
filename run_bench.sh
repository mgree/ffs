#!/bin/sh

set -e

TIMESTAMP=$(date +"%Y%m%d_%H:%M:%S")

NUM_RUNS_DEFAULT=10
usage() {
    exec >&2
    printf "Usage: %s [-e] [-n NUM_RUNS]\n\n" "$(basename $0)"
    printf "       -n NUM_RUNS    the number of runs for each test case (defaults to $NUM_RUNS_DEFAULT)\n"
    printf "       -e             run eager-mode benchmarks, as well\n"
    exit 2
}

ARGS=""
while getopts ":en:h" opt
do
    case "$opt" in
        (e) EAGER=1
            ;;
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

if [ "$EAGER" ]
then
    printf "BENCHMARKING EAGER MODE\n"

    BENCH_EAGER="${TIMESTAMP}/eager_bench.log"
    FFS_ARGS="--eager" ./bench.sh $ARGS >"$BENCH_EAGER"

    FFS_ARGS="--eager" ./bench.sh -d micro $ARGS >"$MICRO_RAW"
    MICRO_EAGER="${TIMESTAMP}/eager_micro.log"
    ./fixup_micro.sh "$MICRO_RAW" >"$MICRO_EAGER"
else
    printf "SKIPPING EAGER MODE\n"
fi

printf "BENCHMARKING WITH WORKLOAD: read_all\n"

BENCH_WORKLOAD="${TIMESTAMP}/read_all_bench.log"
./bench.sh -w workloads/read_all.sh $ARGS >"$BENCH_WORKLOAD"

./bench.sh -d micro -w workloads/read_all.sh $ARGS >"$MICRO_RAW"
MICRO_WORKLOAD="${TIMESTAMP}/read_all_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO_WORKLOAD"

rm "$MICRO_RAW"

./generate_charts.R "$BENCH_LAZY"     "$MICRO_LAZY"
./generate_charts.R "$BENCH_EAGER"    "$MICRO_EAGER"
./generate_charts.R "$BENCH_WORKLOAD" "$MICRO_WORKLOAD"
