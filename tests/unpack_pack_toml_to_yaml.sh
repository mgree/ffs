#/bin/sh

# convert toml to yaml with unpack pack

# unpack from format 1
# pack to format 2
# unpack from format 2
# diff -r unpacked1 unpacked2

fail() {
    echo FAILED: $1
    rm -r "$UNPACK_MNT0"
    rm -r "$UNPACK_MNT1"
    rm "$PACK_FILE0"
    rm "$ERR_MSG"
    exit 1
}

ERR_MSG=$(mktemp)
for f in $(find ../toml -maxdepth 1 -name '*.toml'); do
    UNPACK_MNT0=$(mktemp -d)
    # using `--exact` because datetime object becomes a string in json and adds a newline when unpacked as json.
    unpack $f --exact --into "$UNPACK_MNT0" 2>"$ERR_MSG" || fail unpack1
    # skip the issue where it doesn't unpack into a directory structure
    cat "$ERR_MSG" | grep -i -e "the unpacked form must be a directory" >/dev/null 2>&1 && continue
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    pack "$UNPACK_MNT0" --exact -t yaml >"$PACK_FILE0" || fail pack1
    unpack "$PACK_FILE0" --exact -t yaml --into "$UNPACK_MNT1" || fail unpack2
    [ -z "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ] || fail diff
    rm -r "$UNPACK_MNT0"
    rm -r "$UNPACK_MNT1"
    rm "$PACK_FILE0"
done

rm "$ERR_MSG"
