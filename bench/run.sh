#!/bin/sh

CLEANUP=
CLEANUP_DIR=
PIDS=
tempfile() {
    file=$(mktemp)
    CLEANUP="$CLEANUP $file"
    eval "$1=$file"
}
tempdir() {
    dir=$(mktemp -d)
    CLEANUP_DIR="$CLEANUP $dir"
    eval "$1=$dir"
}

cleanup() {
    for file in $CLEANUP
    do
        [ "$file" ] && [ -f "$file" ] && rm "$file"  >/dev/null 2>&1
    done
    CLEANUP=""

    for dir in $CLEANUP_DIR
    do
        [ "$dir" ] && [ -f "$dir" ] && rmdir "$dir" >/dev/null 2>&1
    done
    CLEANUP_DIR=""

    for pid in $PIDS
    do
        kill $pid >/dev/null 2>&1
    done
    PIDS=""
}

trap 'cleanup' EXIT
trap 'echo "Interrupted!"; cleanup; exit' INT

NUM_RUNS_DEFAULT=10
usage() {
    exec >&2
    printf "Usage: %s [-n NUM_RUNS] [PATTERNS ...]\n\n" "$(basename $0)" 
    printf "       NUM_RUNS    the number of runs for each test case (defaults to $NUM_RUNS_DEFAULT)\n"
    printf "       PATTERNS    regular expression patterns for grep; tests matching any pattern will be run (defaults .*)\n"
    exit 2
}

while getopts ":n:h" opt
do
    case "$opt" in
        (n) if [ $((OPTARG)) -le 0 ]
            then
                printf "NUM_RUNS must be a positive number; got '%s'\n\n" "$OPTARG"
                usage
            fi
            NUM_RUNS=$OPTARG
            ;;
        (h) usage
            ;;
        (*) printf "Unrecognized argument '%s'\n\n" "$OPTARG"
            usage
            ;;
    esac
done
shift $((OPTIND - 1))

if [ $# -ge 1 ]
then
    PATTERN="$1"
    shift
    for kw in "$@"
    do
        PATTERN="$PATTERN\|$kw"
    done
fi

: ${NUM_RUNS=$NUM_RUNS_DEFAULT}
run_digits=${#NUM_RUNS}
: ${FFS=$(dirname $0)/../target/release/ffs}
: ${PATTERN=".*"}

# COLLECT DIRECTORIES
dirs=""
for file in $(ls)
do
    [ -d $file ] && dirs="$dirs $file"
done

# GENERATE PLAN
# each file gets $NUM_RUNS runs, in a random order
tempfile all
tempfile plan

dir_len=0

for d in $dirs
do
    for f in $(ls $d)
    do
        for r in $(seq 1 $NUM_RUNS)
        do
            echo $d,$f,$r >>$all

            path="$d/$f"
            if [ "${#path}" -gt $dir_len ]
            then
                dir_len=${#path}
            fi
        done
    done
done
shuf $all >$plan

# EXECUTE PLAN
tempdir mnt
for entry in $(cat $plan | grep -e "$PATTERN")
do
    d=$(echo $entry | cut -d, -f1)
    f=$(echo $entry | cut -d, -f2)
    r=$(echo $entry | cut -d, -f3)

    path="$d/$f"
    printf "%${dir_len}s (copy %${run_digits}d)\n" "$path" "$r" >&2

    tempfile log
    $FFS --time -m $mnt $path >/dev/null 2>$log &
    PID=$!
    PIDS="$PIDS $PID"
    while ! umount $mnt >/dev/null 2>&1
    do
        sleep 1
    done
    
    count=0
    while kill -0 $PID >/dev/null 2>&1
    do
        sleep $count
        if [ "$count" -le 2 ]
        then
            : $((count += 1))
        else
            echo "warning: $PID still running for $path" >&2
            kill $PID
            break
        fi
    done

    size=$(stat -f %z $path)
    while read line
    do
        printf "%s,%s,%s,%s,%s\n" "$d" "$f" "$r" "$size" "$line"
    done <$log
done
