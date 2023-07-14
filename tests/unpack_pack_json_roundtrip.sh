#/bin/sh

fail() {
    echo FAILED: $1
    rm -r "$UNPACK_MNT0"
    rm -r "$UNPACK_MNT1"
    rm "$PACK_FILE0"
    rm "$PACK_FILE1"
    rm "$ERR_MSG"
    exit 1
}

ERR_MSG=$(mktemp)
for f in ../json/*.json; do
    UNPACK_MNT0=$(mktemp -d)
    unpack $f --into "$UNPACK_MNT0" 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    cat "$ERR_MSG" | grep -i -e "the unpacked form must be a directory" >/dev/null 2>&1 && continue
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    PACK_FILE1=$(mktemp)
    pack "$UNPACK_MNT0" -t json >"$PACK_FILE0"
    unpack "$PACK_FILE0" -t json --into "$UNPACK_MNT1"
    pack "$UNPACK_MNT1" -t json >"$PACK_FILE1"
    [ -z "$(diff $PACK_FILE0 $PACK_FILE1)" ] && [ -z "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ] || fail diff
    rm -r "$UNPACK_MNT0"
    rm -r "$UNPACK_MNT1"
    rm "$PACK_FILE0"
    rm "$PACK_FILE1"
done

rm "$ERR_MSG"
