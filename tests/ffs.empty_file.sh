#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
EMPTY=$(mktemp --suffix=.json)

# Create an empty JSON file
echo -n "" > "$EMPTY"

# Mount the empty file - should create an empty object by default
ffs -m "$MNT" "$EMPTY" &
PID=$!
"$WAITFOR" mount "$MNT"
cd "$MNT"

# Should be an empty directory
[ -z "$(ls)" ] || fail "expected empty directory"

# Add a field
echo "test value" > testfield

cd - >/dev/null 2>&1
"$WAITFOR" umount "$MNT" || fail unmount
"$WAITFOR" exit $PID

kill -0 $PID >/dev/null 2>&1 && fail process

# Check the output contains the new field
grep -q "testfield" "$EMPTY" || fail "output should contain testfield"

rm "$EMPTY"
rmdir "$MNT" || fail mount
