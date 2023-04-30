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
for f in ../yaml/*.yaml; do
    UNPACK_MNT0=$(mktemp -d)
    unpack $f --into $UNPACK_MNT0 2>"$ERR_MSG"
    # skip the issue where it doesn't unpack into a directory structure
    if [ "$(cat $ERR_MSG | grep -i "the unpacked form must be a directory" >/dev/null 2>&1)" ]
    then
        continue
    fi
    PACK_FILE0=$(mktemp)
    UNPACK_MNT1=$(mktemp -d)
    PACK_FILE1=$(mktemp)
    pack $UNPACK_MNT0 -t yaml > $PACK_FILE0
    unpack $PACK_FILE0 -t yaml --into $UNPACK_MNT1
    pack $UNPACK_MNT1 -t yaml > $PACK_FILE1
    if [ -n "$(diff $PACK_FILE0 $PACK_FILE1)" || -n "$(diff -r $UNPACK_MNT0 $UNPACK_MNT1)" ]
    then
        fail diff
    fi
    rm -r $UNPACK_MNT0
    rm -r $UNPACK_MNT1
    rm $PACK_FILE0
    rm $PACK_FILE1
done

rm $ERR_MSG
