#!/bin/sh

TIMEOUT="$(cd ../utils; pwd)/timeout"
WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

# --- JSON empty file ---
MNT=$(mktemp -d)

ffs -m "$MNT" ../json/empty.json &
PID=$!
"$WAITFOR" mount "$MNT"

[ -z "$(ls "$MNT")" ] || fail json_notempty
"$WAITFOR" umount "$MNT" || fail json_unmount
"$WAITFOR" exit $PID
kill -0 $PID >/dev/null 2>&1 && fail json_process
rmdir "$MNT" || fail json_mount

# --- TOML empty file ---
MNT=$(mktemp -d)

ffs -m "$MNT" ../toml/empty.toml &
PID=$!
"$WAITFOR" mount "$MNT"

[ -z "$(ls "$MNT")" ] || fail toml_notempty
"$WAITFOR" umount "$MNT" || fail toml_unmount
"$WAITFOR" exit $PID
kill -0 $PID >/dev/null 2>&1 && fail toml_process
rmdir "$MNT" || fail toml_mount

# --- YAML empty file ---
MNT=$(mktemp -d)

ffs -m "$MNT" ../yaml/empty.yaml &
PID=$!
"$WAITFOR" mount "$MNT"

[ -z "$(ls "$MNT")" ] || fail yaml_notempty
"$WAITFOR" umount "$MNT" || fail yaml_unmount
"$WAITFOR" exit $PID
kill -0 $PID >/dev/null 2>&1 && fail yaml_process
rmdir "$MNT" || fail yaml_mount

# --- --strict should error on empty JSON ---
MNT=$(mktemp -d)

"$TIMEOUT" -t 2 ffs --strict -m "$MNT" ../json/empty.json 2>/dev/null
[ $? -ne 0 ] || fail strict_should_error
rmdir "$MNT" || fail strict_mount
