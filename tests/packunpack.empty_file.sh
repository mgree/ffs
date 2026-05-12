#!/bin/sh

TIMEOUT="$(cd ../utils; pwd)/timeout"

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -r "$MNT"
    fi
    exit 1
}

# --- JSON empty file ---
MNT=$(mktemp -d)
unpack --into "$MNT" ../json/empty.json || fail json_unpack
[ -z "$(ls "$MNT")" ] || fail json_notempty
rm -r "$MNT" || fail json_cleanup

# --- TOML empty file ---
MNT=$(mktemp -d)
unpack --into "$MNT" ../toml/empty.toml || fail toml_unpack
[ -z "$(ls "$MNT")" ] || fail toml_notempty
rm -r "$MNT" || fail toml_cleanup

# --- YAML empty file ---
MNT=$(mktemp -d)
unpack --into "$MNT" ../yaml/empty.yaml || fail yaml_unpack
[ -z "$(ls "$MNT")" ] || fail yaml_notempty
rm -r "$MNT" || fail yaml_cleanup

# --- --strict should error on empty JSON ---
MNT=$(mktemp -d)
"$TIMEOUT" -t 2 unpack --strict --into "$MNT" ../json/empty.json 2>/dev/null
[ $? -ne 0 ] || fail strict_should_error
rm -r "$MNT" || fail strict_cleanup
