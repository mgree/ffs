#!/bin/sh
#
# from https://github.com/mgree/ffs/issues/42

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        umount "$D"/single
        rm -r "$D"
    fi
    exit 1
}

TESTS="$(pwd)"

D=$(mktemp -d)

cp ../json/single.json "$D"/single.json
cp ../json/false.json "$D"/false.json

cd "$D"

cp single.json unreadable.json
chmod -r unreadable.json

mkdir unwriteable
chmod -w unwriteable

#### ERROR_STATUS_FUSE
## UNPACK
# mount exists but unempty
mkdir -p unempty/dir
unpack --into unempty single.json 2>single.err
[ $? -eq 1 ] || fail unempty_mount_status
[ -s single.err ] || fail unempty_mount_msg
rm single.err

# inferred mount already exists, use --into
mkdir single
unpack single.json 2>single.err
[ $? -eq 1 ] || fail inferred_mount_exists_status
[ -s single.err ] || fail inferred_mount_exists_msg
rm single.err

# mount unwriteable
cd unwriteable
unpack single.json 2>../single.err
[ $? -eq 1 ] || fail unwriteable_mount_status
cd ..
[ -s single.err ] || fail unwriteable_mount_msg
rm single.err

# input file doesn't exist
unpack nonesuch.toml 2>nonesuch.err
[ $? -eq 1 ] || fail input_dne_status
[ -s nonesuch.err ] || fail input_dne_msg
rm nonesuch.err

# input file unreadable
unpack unreadable.json 2>ur.err
[ $? -eq 1 ] || fail unreadable_status
[ -s ur.err ] || fail unreadable_msg
rmdir unreadable
rm ur.err

# input is .., couldn't infer mount
unpack .. 2>dotdot.err
[ $? -eq 1 ] || fail dotdot_infer_mount_status
[ -s dotdot.err ] || fail dotdot_infer_mount_msg
rm dotdot.err

# plain value input, already tested with null in bad_root.sh
unpack false.json 2>false.err
[ $? -eq 1 ] || fail false_bad_root_status
[ -s false.err ] || fail false_bad_root_msg
rm false.err

## PACK
# input directory doesn't exist
pack nonesuch 2>nonesuch.err
[ $? -eq 1 ] || fail pack_no_input_dir_status
[ -s nonesuch.err ] || fail pack_no_input_dir_msg
rm nonesuch.err

#### ERROR_STATUS_CLI
# unpack input is stdin but no mount specified
echo '{}' | unpack - 2>stdin.err
[ $? -eq 2 ] || fail stdin_nomount_status
[ -s stdin.err ] || fail stdin_nomount_msg
rm stdin.err

# pack directory not specified
pack 2>nodir.err
[ $? -eq 2 ] || fail pack_no_dir_status
[ -s nodir.err ] || fail pack_no_dir_msg
rm nodir.err

# bad shell completions
unpack --completions smoosh 2>comp.err
[ $? -eq 2 ] || fail unpack_comp_unsupported_shell_status
[ -s comp.err ] || fail unpack_comp_unsupported_shell_msg
rm comp.err

pack --completions smoosh 2>comp.err
[ $? -eq 2 ] || fail pack_comp_unsupported_shell_status
[ -s comp.err ] || fail pack_comp_unsupported_shell_msg
rm comp.err

# unknown unpack --type
unpack --type hieratic single.json 2>type.err
[ $? -eq 2 ] || fail unpack_unknown_type_status
[ -s type.err ] || fail unpack_unknown_type_msg
rm type.err

# unknown pack --target
unpack --into unk_tgt single.json
pack --target hieratic unk_tgt 2>target.err
[ $? -eq 2 ] || fail pack_unknown_target_status
[ -s target.err ] || fail pack_unknown_target_msg
rm target.err


chmod +w unwriteable
cd "$TESTS"
rm -r "$D" || fail cleanup
