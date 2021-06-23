#!/bin/sh

fail() {
    echo FAILED: $1
    if [ "$MNT" ]
    then
        cd
        umount "$MNT"
        rmdir "$MNT"
        rm "$ERR"
    fi
    exit 1
}

MNT=$(mktemp -d)
ERR=$(mktemp)

RUST_LOG="ffs=debug" ffs -d --no-output "$MNT" ../json/object.json &
PID=$!
sleep 2
chown :nobody "$MNT"/name 2>$ERR >&2 && fail "chgrp1: $(cat $ERR)"
[ -s "$ERR" ] || fail "chgrp1 error: $(cat $ERR)"
groups
ls -l "$MNT"/name
echo $(groups | cut -d' ' -f 1)
chown :$(groups | cut -d' ' -f 1) "$MNT"/name 2>$ERR >&2 || fail "chgrp2: $(cat $ERR)"
[ -s "$ERR" ] && fail "chgrp2 error: $(cat $ERR)"
chown $(whoami) "$MNT"/name 2>$ERR >&2 || fail chown
[ -s "$ERR" ] && fail "chown error: $(cat $ERR)"
umount "$MNT" || fail unmount1    
sleep 1
kill -0 $PID >/dev/null 2>&1 && fail process1

rmdir "$MNT" || fail mount
rm "$ERR"

