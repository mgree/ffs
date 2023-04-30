#/bin/sh

# convert json to toml with unpack pack

# unpack from format 1
# pack to format 2
# unpack from format 2
# diff -r unpacked1 unpacked2

fail() {
    echo FAILED: $1
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
    rm $ERR_MSG
    exit 1
}

ERR_MSG=$(mktemp)
# reasons for skipping:
# json_eg5.json has null values
# list.json and list2.json are lists at the top level, which toml doesn't support
# object_null.json has a null value
for f in $(find ../json -maxdepth 1 -name '*.json' ! -name 'json_eg5.json' ! -name 'list*.json' ! -name 'object_null.json' | sort); do
    UNPACK_MNT0=$(mktemp -d)
    unpack $f --into $UNPACK_MNT0 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    cat $ERR_MSG | grep -i -e "the unpacked form must be a directory" >/dev/null 2>&1 && continue
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    pack $UNPACK_MNT0 -t toml > $PACK_FILE0
    unpack $PACK_FILE0 -t toml --into $UNPACK_MNT1
    [ -z "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ] || fail diff
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
done

rm $ERR_MSG
