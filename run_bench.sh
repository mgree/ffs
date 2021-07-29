#!/bin/sh

TIMESTAMP=$(date +"%Y%M%d_%H:%M:%S")

cd bench

BENCH="../${TIMESTAMP}_bench.log"
./bench.sh >"$BENCH"

./mk_micro.sh
MICRO="../${TIMESTAMP}_micro.log"
./bench.sh -d micro >"$MICRO"

./generate_charts.R "$BENCH" "$MICRO"
