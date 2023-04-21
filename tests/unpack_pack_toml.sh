#/bin/sh

fail() {
    echo FAILED: $1
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
    rm $PACK_FILE1
    rm $ERR_MSG
    exit 1
}

ERR_MSG=$(mktemp)
for f in ../toml/*.toml; do
    UNPACK_MNT0=$(mktemp -d)
    # TODO (nad) 2023-04-21 remove unnecessary comments
    # echo "testing $f"
    ../target/debug/unpack $f --into $UNPACK_MNT0 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    cat $ERR_MSG | grep -i -e "the unpacked form must be a directory" >/dev/null 2>&1 && {
        # echo "skipping: the unpacked form must be a directory"
        continue
    }
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    PACK_FILE1=$(mktemp)
    ../target/debug/pack $UNPACK_MNT0 -t toml > $PACK_FILE0
    ../target/debug/unpack $PACK_FILE0 -t toml --into $UNPACK_MNT1
    ../target/debug/pack $UNPACK_MNT1 -t toml > $PACK_FILE1
    [[ -z `diff $PACK_FILE0 $PACK_FILE1` && -z `diff -r $UNPACK_MNT0 $UNPACK_MNT1` ]] || fail diff
    # TODO (nad) 2023-04-13 think about how to remove tmp files safely
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
    rm $PACK_FILE1
done
