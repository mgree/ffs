#/bin/sh

# convert yaml to json with unpack pack

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
# reasons for skipping:
# invoice.yaml's floats with .00 become .0 in json. Everything else is perfect after unpacking the packed json version
for f in $(find ../yaml -maxdepth 1 -name '*.yaml' ! -name 'invoice.yaml'); do
    UNPACK_MNT0=$(mktemp -d)
    unpack $f --into "$UNPACK_MNT0" 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    cat "$ERR_MSG" | grep -i -e "the unpacked form must be a directory" >/dev/null 2>&1 && continue
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    pack "$UNPACK_MNT0" -t json >"$PACK_FILE0" || fail pack1
    unpack "$PACK_FILE0" -t json --into "$UNPACK_MNT1" || fail unpack2
    [ -z "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ] || fail diff
    rm -r "$UNPACK_MNT0"
    rm -r "$UNPACK_MNT1"
    rm "$PACK_FILE0"
done

rm "$ERR_MSG"
