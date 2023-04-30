#/bin/sh

# convert toml to json with unpack pack

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
for f in $(find ../toml -maxdepth 1 -name '*.toml' | sort); do
    UNPACK_MNT0=$(mktemp -d)
    # using `--exact` because datetime object becomes a string in json and adds a newline when unpacked as json.
    unpack $f --exact --into $UNPACK_MNT0 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    if [ "$(cat $ERR_MSG | grep -i "the unpacked form must be a directory" >/dev/null 2>&1)" ]
    then
        continue
    fi
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    pack $UNPACK_MNT0 --exact -t json > $PACK_FILE0
    unpack $PACK_FILE0 --exact -t json --into $UNPACK_MNT1
    if [ -n "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ]
    then
        fail diff
    fi
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
done

rm $ERR_MSG
