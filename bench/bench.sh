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

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    filesize() {
        stat --printf=%s $1
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    filesize() {
        stat -f "%z" $1
    }
else
    echo "The benchmark suite only runs on macOS and Linux." >&2
    exit 3
fi

trap 'cleanup' EXIT
trap 'echo "Interrupted!"; cleanup; exit' INT

NUM_RUNS_DEFAULT=10
usage() {
    exec >&2
    printf "Usage: %s [-n NUM_RUNS] [-d DIR] [PATTERNS ...]\n\n" "$(basename $0)"
    printf "       -d DIR         runs tests in DIR only\n"
    printf "       -n NUM_RUNS    the number of runs for each test case (defaults to $NUM_RUNS_DEFAULT)\n"
    printf "       PATTERNS       regular expression patterns for grep; tests matching any pattern will be run (defaults .*)\n"
    exit 2
}

while getopts ":n:d:h" opt
do
    case "$opt" in
        (n) if [ $((OPTARG)) -le 0 ]
            then
                printf "NUM_RUNS must be a positive number; got '%s'\n\n" "$OPTARG"
                usage
            fi
            NUM_RUNS=$OPTARG
            ;;
        (d) if ! [ -d "$OPTARG" ]
            then
                printf "No such directory '%s'." "$OPTARG"
                exit 1
            fi
            DIRS="$OPTARG"
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

TIMEOUT="$(cd ../utils; pwd)/timeout"

: ${NUM_RUNS=$NUM_RUNS_DEFAULT}
run_digits=$(( ${#NUM_RUNS} ))
: ${FFS=$(dirname $0)/../target/release/ffs}
: ${PATTERN=".*"}

: ${DIRS=doi fda gh gov.uk json.org ncdc penguin penn rv synthetic}

# GENERATE PLAN
# each file gets $NUM_RUNS runs, in a random order
tempfile all
tempfile plan

dir_len=0

for d in $DIRS
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
# randomize, and then sort by 'run' number
sort -R $all | sort -k 3 -t , -s -n >$plan

total_runs=$(( $(cat $plan | wc -l) ))
total_digits=${#total_runs}

# EXECUTE PLAN
tempdir mnt

printf "source,file,run,size,activity,ns\n" "$d" "$f" "$r" "$size" "$line" # header

errors=""
has_error() {
    case "$errors" in
        (*$1*) return 0;;
    esac
    return 1
}
errored() {
    if ! has_error $1
    then
        errors="$errors $1"
    fi
    kill $PID >/dev/null 2>&1
    [ -d $mnt ] && umount $mnt >/dev/null 2>&1
    [ -f $log ] && rm $log
}

run=0
for entry in $(cat $plan | grep -e "$PATTERN")
do
    d=$(echo $entry | cut -d, -f1)
    f=$(echo $entry | cut -d, -f2)
    r=$(echo $entry | cut -d, -f3)

    path="$d/$f"

    : $((run += 1))
    printf "%${total_digits}d/%d  %${dir_len}s " "$run" "$total_runs" "$path" >&2
    if has_error "$path"
    then
        printf "(errored earlier; skipping)\n" >&2
        continue
    else
        printf "(copy %${run_digits}d)\n" "$r" >&2
    fi

    tempfile log
    $FFS --time -m $mnt $path >/dev/null 2>$log &
    PID=$!
    PIDS="$PIDS $PID"
    count=0
    while ! umount $mnt >/dev/null 2>&1
    do
        sleep $count
        if [ "$count" -le 5 ]
        then
            : $((count += 1))
        else
            printf "%$((2 * total_digits + 1))s  warning: couldn't unmount for $path\n" ' ' >&2
            errored $path
            continue 2
        fi
    done
    
    count=0
    while kill -0 $PID >/dev/null 2>&1
    do
        sleep $count
        if [ "$count" -le 2 ]
        then
            : $((count += 1))
        else
            printf "%$((2 * total_digits + 1))s  warning: $PID still running for $path" ' ' >&2
            errored $path
            continue 2
        fi
    done

    size=$(filesize $path)
    while read line
    do
        printf "%s,%s,%s,%s,%s\n" "$d" "$f" "$r" "$size" "$line"
    done <$log
done
