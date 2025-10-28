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
TIMEOUT="$(cd ../utils; pwd)/timeout"

D=$(mktemp -d)

cp ../json/single.json "$D"/single.json
cp ../json/false.json "$D"/false.json

cd "$D"

cp single.json unreadable.json
chmod -r unreadable.json

mkdir unwriteable
chmod -w unwriteable

# in place mount, mountpoint exists
mkdir single
"$TIMEOUT" -t 2 ffs -i single.json 2>single.err
[ $? -eq 1 ] || fail imountstatus
[ -s single.err ] || fail imountmsg
rmdir single
rm single.err

# in place, can't make mountpoint
cd unwriteable
"$TIMEOUT" -t 2 ffs -i single.json 2>../single.err
[ $? -eq 1 ] || fail imkmountstatus
cd ..
[ -s single.err ] || fail imkmountmsg
rm single.err

# new, mountpoint exists
mkdir foo
"$TIMEOUT" -t 2 ffs --new foo.json 2>foo.err
[ $? -eq 1 ] || fail newmountstatus
[ -s foo.err ] || fail newmountmsg
rmdir foo
rm foo.err

# new, can't make mountpoint
cd unwriteable
"$TIMEOUT" -t 2 ffs --new foo.json 2>../foo.err
[ $? -eq 1 ] || fail newmkmountstatus
cd ..
[ -s foo.err ] || fail newmkmountmsg
rm foo.err

# input file, can't infer mountpoint
"$TIMEOUT" -t 2 ffs --new .. 2>dotdot.err
[ $? -eq 1 ] || fail newdotdotmountstatus
[ -s dotdot.err ] || fail newdotdotmountmsg
rm dotdot.err

# --new, output file exists
touch foo.yaml
"$TIMEOUT" -t 2 ffs --new foo.yaml 2>foo.err
[ $? -eq 1 ] || fail omountstatus
[ -s foo.err ] || fail omountmsg
rm foo.yaml
rm foo.err

# --new, mountpoint doesn't exist
"$TIMEOUT" -t 2 ffs -m notthere --new foo.json 2>foo.err
[ $? -eq 1 ] || fail mmountstatus1
[ -s foo.err ] || fail mmountmsg1
rm foo.err

# mountpoint doesn't exist
"$TIMEOUT" -t 2 ffs -m notthere single.json 2>single.err
[ $? -eq 1 ] || fail mmountstatus2
[ -s single.err ] || fail mmountmsg2
rm single.err

# input file doesn't exists
"$TIMEOUT" -t 2 ffs nonesuch.toml 2>nonesuch.err
[ $? -eq 1 ] || fail inputmountstatus1
[ -s nonesuch.err ] || fail inputmountmsg1
rm nonesuch.err

# input file, mountpoint exists
mkdir single
"$TIMEOUT" -t 2 ffs single.json 2>single.err
[ $? -eq 1 ] || fail inputmountstatus2
[ -s single.err ] || fail inputmountmsg2
rmdir single
rm single.err

# input file, can't make mount point
cd unwriteable
"$TIMEOUT" -t 2 ffs ../single.json 2>../single.err
[ $? -eq 1 ] || fail inputmkmountstatus
cd ..
[ -s single.err ] || fail inputmkmountmsg
rm single.err

# input file, can't infer mountpoint
"$TIMEOUT" -t 2 ffs .. 2>dotdot.err
[ $? -eq 1 ] || fail inputdotdotmountstatus
[ -s dotdot.err ] || fail inputdotdotmountmsg
rm dotdot.err

# unreadable input
"$TIMEOUT" -t 2 ffs unreadable.json 2>ur.err
[ $? -eq 1 ] || fail unreadablemountstatus
[ -s ur.err ] || fail unreadablemountmsg
rm ur.err

# plain value input
"$TIMEOUT" -t 2 ffs false.json 2>false.err
[ $? -eq 1 ] || fail falsemountstatus
[ -s false.err ] || fail falsemountmsg
rm false.err

# bad mount point (fuser is masking this error)
# "$TIMEOUT" -t 2 ffs /etc single.json 2>etc.err
# [ $? -eq 1 ] || fail etcmountstatus
# [ -s etc.err ] || fail etcmountmsg
# rm etc.err

# bad shell completion
"$TIMEOUT" -t 2 ffs --completions smoosh 2>comp.err
[ $? -eq 2 ] || fail compmountstatus
[ -s comp.err ] || fail compmountmsg
rm comp.err

# bad mode
"$TIMEOUT" -t 2 ffs --mode 888 2>mode.err
[ $? -eq 2 ] || fail modemountstatus
[ -s mode.err ] || fail modemountmsg
rm mode.err

# bad dirmode
"$TIMEOUT" -t 2 ffs --dirmode 888 2>dirmode.err
[ $? -eq 2 ] || fail dirmodemountstatus
[ -s dirmode.err ] || fail dirmodemountmsg
rm dirmode.err

# new and input file
"$TIMEOUT" -t 2 ffs --new foo.json single.json 2>ni.err
[ $? -eq 2 ] || fail nimountstatus
[ -s ni.err ] || fail nimountmsg
rm ni.err

# unknown --source
"$TIMEOUT" -t 2 ffs --source hieratic single.json 2>source.err
[ $? -eq 2 ] || fail sourcemountstatus
[ -s source.err ] || fail sourcemountmsg
rm source.err

# unknown --target
"$TIMEOUT" -t 2 ffs --target hieratic single.json 2>target.err
[ $? -eq 2 ] || fail targetmountstatus
[ -s target.err ] || fail targetmountmsg
rm target.err

# stdin read, no mountpoint
"$TIMEOUT" -t 2 ffs 2>im.err
[ $? -eq 2 ] || fail immountstatus
[ -s im.err ] || fail immountmsg
rm im.err

chmod +w unwriteable
cd "$TESTS"
rm -r "$D" || fail cleanup
