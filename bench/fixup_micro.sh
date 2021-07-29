#!/bin/sh

# the log format for micro benchmark has a bunch of information nested in the filename
# here we break it apart so that we can generate appropriate charts

[ "$#" -eq 2 ] && [ -f "$1" ] || {
    echo "Usage: $(basename $0) [BENCHMARK LOG]"    >&2
    echo                                            >&2
    echo "       see run_bench.sh in the repo root" >&2
    exit 2
}

group=$(mktemp)
name=$(mktemp)
info=$(mktemp)
rest=$(mktemp)

cat $1 | cut -f 1 -d ',' >$group
cat $1 | cut -f 2 -d ',' >$name

# take the filename and break it up; we need to emit a new header field, too
echo kind,direction,magnitude >$info
tail -n +2 $name | sed s/_/,/g | sed s/.json// >>$info

cat $1 | cut -f 3,4,5,6 -d ',' >$rest

paste -d ',' $group $name $info $rest

rm $group $name $info $rest
