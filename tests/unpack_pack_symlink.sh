#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        rm "$EXP" "$OUT"
        rm -r "$MNT"
    fi
    exit 1
}

if [ "$RUNNER_OS" = "Linux" ] || [ "$(uname)" = "Linux" ]; then
    which getfattr || fail getfattr
    which setfattr || fail setfattr
    getattr() {
        attr=$1
        shift
        getfattr -h -n "$attr" --only-values "$@"
    }
    setattr() {
        attr="$1"
        val="$2"
        shift 2
        setfattr -h -n "$attr" -v "$val" "$@"
    }
    listattr() {
        getfattr -h --match=- "$@"
    }
    rmattr() {
        attr=$1
        shift
        setfattr -h -x "$attr" "$@"
    }
elif [ "$RUNNER_OS" = "macOS" ] || [ "$(uname)" = "Darwin" ]; then
    listattr() {
        xattr -s -l "$@"
    }
    getattr() {
        attr=$1
        shift
        xattr -s -p "$attr" "$@"
    }
    setattr() {
        attr="$1"
        val="$2"
        shift 2
        xattr -s -w "$attr" "$val" "$@"
    }
    rmattr() {
        attr=$1
        shift
        xattr -s -d "$attr" "$@"
    }
else
    fail os
fi

typeof() {
    getattr user.type "$@"
}

MNT=$(mktemp -d)
EXP=$(mktemp)
OUT=$(mktemp)

# chain of symlinks and symlink to directory
# test0
# ├── a
# ├── b -> a
# ├── c -> b
# ├── d -> c
# ├── e -> d
# ├── tree
# │  ├── about
# │  └── root
# └── treecopy -> tree

cd "$MNT"
echo 'a' >a
ln -s a b
ln -s b c
ln -s c d
ln -s d e
mkdir tree
ln -s tree treecopy
cd tree
echo 'tree about' >about
echo 'tree root' >root

printf '{"a":"a","tree":{"about":"tree about","root":"tree root"}}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack1
diff "$EXP" "$OUT" || fail "test0 no-follow"

printf '{"a":"a","b":"a","c":"a","d":"a","e":"a","tree":{"about":"tree about","root":"tree root"},"treecopy":{"about":"tree about","root":"tree root"}}' >"$EXP"
pack -o "$OUT" -L -- "$MNT" || fail pack2
diff "$EXP" "$OUT" || fail "test0 follow"

printf '{"a":"a","d":"a","tree":{"about":"tree about","root":"tree root"}}' >"$EXP"
pack -o "$OUT" -H "$MNT"/d -- "$MNT" || fail pack3
diff "$EXP" "$OUT" || fail "test0 follow-specified"

rm -r "$MNT"
mkdir "$MNT"

# symlinks in list directories
# test1
# ├── ascending
# │  ├── 0 -> 1
# │  ├── 1 -> 2
# │  ├── 2 -> 3
# │  ├── 3 -> 4
# │  └── 4
# └── descending
#    ├── 0
#    ├── 1 -> 0
#    ├── 2 -> 1
#    ├── 3 -> 2
#    └── 4 -> 3

cd "$MNT"
mkdir ascending descending
cd ascending
echo '4' >4
ln -s 4 3
ln -s 3 2
ln -s 2 1
ln -s 1 0
cd ../descending
echo '0' >0
ln -s 0 1
ln -s 1 2
ln -s 2 3
ln -s 3 4

printf '{"ascending":[4],"descending":[0]}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack4
diff "$EXP" "$OUT" || fail "test1 no-follow"

printf '{"ascending":[4,4,4,4,4],"descending":[0,0,0,0,0]}' >"$EXP"
pack -o "$OUT" -L -- "$MNT" || fail pack5
diff "$EXP" "$OUT" || fail "test1 follow"

printf '{"ascending":[4,4],"descending":[0,0]}' >"$EXP"
pack -o "$OUT" -H "$MNT"/ascending/2 "$MNT"/descending/3 -- "$MNT" || fail pack6
diff "$EXP" "$OUT" || fail "test1 follow-specified"

rm -r "$MNT"
mkdir "$MNT"

# relative and absolute path symlinks to some path in mount
# test2
# └── path
#    └── to
#       ├── other
#       │  └── file
#       │     └── data
#       └── some
#          └── link
#             ├── abs -> "$MNT"/path/to/other/file/data
#             └── rel -> ../../other/file/data

cd "$MNT"
mkdir -p path/to/some/link path/to/other/file
touch path/to/other/file/data
cd path/to/some/link
ln -s ../../other/file/data rel
ln -s "$MNT"/path/to/other/file/data abs

printf '{"path":{"to":{"other":{"file":{"data":null}},"some":{"link":{}}}}}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack7
diff "$EXP" "$OUT" || fail "test2 no-follow"

printf '{"path":{"to":{"other":{"file":{"data":null}},"some":{"link":{"abs":null,"rel":null}}}}}' >"$EXP"
pack -o "$OUT" -L -- "$MNT" || fail pack8
diff "$EXP" "$OUT" || fail "test2 follow"

printf '{"path":{"to":{"other":{"file":{"data":null}},"some":{"link":{"abs":null}}}}}' >"$EXP"
pack -o "$OUT" -H "$MNT"/path/to/some/link/abs -- "$MNT" || fail pack9
diff "$EXP" "$OUT" || fail "test2 follow-specified"

rm -r "$MNT"
mkdir "$MNT"

# symlink pointing to ancestor error
# test3
# └── path
#    └── to
#       ├── other
#       │  └── file
#       │     └── data
#       └── some
#          └── link
#             └── linkfile -> ../../some

cd "$MNT"
mkdir -p path/to/some/link path/to/other/file
touch path/to/other/file/data
cd path/to/some/link
ln -s ../../some linkfile

printf '{"path":{"to":{"other":{"file":{"data":null}},"some":{"link":{}}}}}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack10
diff "$EXP" "$OUT" || fail "test3 no-follow"

pack -L -- "$MNT" >/dev/null 2>"$OUT" && fail "pack11 symlink to ancestor error"
cat "$OUT" | grep "ancestor directory" >/dev/null 2>&1 || fail "test3 follow expected error"

rm -r "$MNT"
mkdir "$MNT"

# symlink loop
# test4
# ├── a -> b
# ├── b -> a
# ├── c -> b
# ├── d -> c
# ├── e -> d
# └── f -> e

cd "$MNT"
ln -s a b
ln -s b c
ln -s c d
ln -s d e
ln -s e f
ln -s b a

printf '{}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack12
diff "$EXP" "$OUT" || fail "test4 no-follow"

pack -L -- "$MNT" >/dev/null 2>"$OUT" && fail "pack13 symlink loop error"
cat "$OUT" | grep "Symlink loop detected" >/dev/null 2>&1 || fail "test4 follow expected error"

rm -r "$MNT"
mkdir "$MNT"

# xattr propagates up the symlink chain unless redefined
# test5
# ├── a
# ├── b -> a
# ├── c -> b
# ├── d -> c
# ├── e -> d
# └── f -> e

cd "$MNT"
echo '4' >a
ln -s a b
ln -s b c
ln -s c d
ln -s d e
ln -s e f
setattr user.type integer a
setattr user.type string c
setattr user.type bytes e
typeof a
typeof b
typeof c
typeof d
typeof e
typeof f

printf '{"a":4}' >"$EXP"
pack -o "$OUT" -- "$MNT" || fail pack14
diff "$EXP" "$OUT" || fail "test5 no-follow"

printf '{"a":4,"b":4,"c":"4","d":"4","e":"NAo=","f":"NAo="}' >"$EXP"
pack -o "$OUT" -L -- "$MNT" || fail pack15
diff "$EXP" "$OUT" || fail "test5 follow"

# TODO(nad) 2023-08-08: maybe only xattrs from followed symlinks are allowed.
printf '{"a":4,"b":4,"d":"4","f":"NAo="}' >"$EXP"
pack -o "$OUT" -H "$MNT"/b "$MNT"/d "$MNT"/f -- "$MNT" || fail pack16
diff "$EXP" "$OUT" || fail "test5 follow-specified"

rm "$EXP" "$OUT"
rm -r "$MNT"
