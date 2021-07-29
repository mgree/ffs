#!/bin/sh

TIMESTAMP=$(date +"%Y%M%d_%H:%M:%S")

cd bench

BENCH="../${TIMESTAMP}_bench.log"
./bench.sh >"$BENCH"

./mk_micro.sh
MICRO_RAW=$(mktemp)
./bench.sh -d micro >"$MICRO"

MICRO="../${TIMESTAMP}_micro.log"
./fixup_micro.sh "$MICRO_RAW" >"$MICRO"

./generate_charts.R "$BENCH" "$MICRO"
