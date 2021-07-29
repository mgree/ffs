#!/bin/sh

set -e

TIMESTAMP=$(date +"%Y%m%d_%H:%M:%S")

cd bench

BENCH="../${TIMESTAMP}_bench.log"
./bench.sh -n 1 >"$BENCH"

./mk_micro.sh
MICRO_RAW=$(mktemp)
./bench.sh -d micro -n 1 >"$MICRO_RAW"
MICRO="../${TIMESTAMP}_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO"
rm "$MICRO_RAW"

./generate_charts.R "$BENCH" "$MICRO"
