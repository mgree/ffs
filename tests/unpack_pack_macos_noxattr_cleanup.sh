#!/bin/sh

if ! [ "$RUNNER_OS" = "macOS" ] && ! [ "$(uname)" = "Darwin" ]
then
    echo "This test only runs under macOS; you're using ${RUNNER_OS-$(uname)}" >&2
    exit 0
fi

VERSION="$( sw_vers -productVersion | cut -d. -f1 )"
pre_ventura_test() {
    if [ $VERSION -lt 13 ]
    then
        true
    else
        false
    fi
}
non_macosdot_filesystem_test() {
    TESTDIR=$(mktemp -d)
    touch "$TESTDIR"/testfile
    xattr -w xattr_test xattr_test "$TESTDIR"/testfile
    if ! [ -e "$TESTDIR"/._testfile ]
    then
        rm -r "$TESTDIR"
        true
    else
        rm -r "$TESTDIR"
        false
    fi
}

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm -rf "$MNT"
        rm "$OUT"
    fi
    exit 1
}

listattr() {
    xattr -l "$@"
}
getattr() {
    attr=$1
    shift
    xattr -p "$attr" "$@"
}
setattr() {
    attr="$1"
    val="$2"
    shift 2
    xattr -w "$attr" "$val" "$@"
}
rmattr() {
    attr=$1
    shift
    xattr -d "$attr" "$@"
}

typeof() {
    getattr user.type "$@"
}

MNT=$(mktemp -d)
OUT=$(mktemp)

unpack --into "$MNT" --no-xattr ../json/object.json

[ "$(typeof $MNT)"             = "named"   ] && fail root
[ "$(typeof $MNT/name)"        = "string"  ] && fail name
[ "$(typeof $MNT/eyes)"        = "float"   ] && fail eyes
[ "$(typeof $MNT/fingernails)" = "float"   ] && fail fingernails
[ "$(typeof $MNT/human)"       = "boolean" ] && fail human

setattr user.type list "$MNT" || fail set1

[ "$(typeof $MNT)" = "list"   ] || fail "macos override"

pack -o "$OUT" --no-xattr --target json "$MNT"

# for all the grep tests in this file, instead of looking for the literal "._.",
# look for any strings beginning with ._
grep -e '"\._.*"' "$OUT" >/dev/null 2>&1 && fail metadata1

rm -rf "$MNT"
rm "$OUT"

# now try to keep the metadata
unpack --into "$MNT" --no-xattr ../json/object.json
setattr user.type list "$MNT"

pack -o "$OUT" --no-xattr --keep-macos-xattr --target json "$MNT"

# ffs creates a literal ._. file because it can't store the xattr of the root of the fuse filesystem
# outside the mount. Therefore, there is only one xattr (._.) in the output for the 2nd test for ffs.
#
# For OS version >= 13 and on filesystems that create ._ files for xattrs, the com.apple.provenance
# xattr is automatically created despite using --no-xattr for unpack. So, pack will find these pointless
# ._ files and include them. Technically, this means the grep command should be successful.
# However, what this means for OS version < 13 is that there should be no ._ files created by unpack and the
# setattr command above should only create the ._(name of unpacked directory) file, outside the directory
# in which the data is unpacked. So, pack will never see ._(name of unpacked directory) and will only
# output the 4 json attributes inside the json object (eyes, fingernails, human, name).
# So, the grep command should fail.
#
# If the grep fails, then if OS version < 13, the ._files were not created by default, so the test passes.
# Otherwise, if the current filesystem doesn't create ._files for xattrs at all, then the test passes.
# Otherwise, the test fails.
grep -e '"\._.*"' "$OUT" >/dev/null 2>&1 || pre_ventura_test || non_macosdot_filesystem_test || fail metadata2

rm -rf "$MNT"
rm "$OUT"

# now try to keep the metadata but also have the FS store it
unpack --into "$MNT" ../json/object.json

setattr user.type list "$MNT"

pack -o "$OUT" --keep-macos-xattr --target json "$MNT"

# technically, the output of pack here still differs from that of ffs on macosdot filesystems because ffs doesn't
# create xattrs for (eyes, fingernails, human, name).
grep -e '"\._.*"' "$OUT" >/dev/null 2>&1 && fail metadata3

rm -rf "$MNT" || fail mount
rm "$OUT"
