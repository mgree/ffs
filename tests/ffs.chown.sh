#!/bin/sh

WAITFOR="$(cd ../utils; pwd)/waitfor"
. ./fail.def

MNT=$(mktemp -d)
ERR=$(mktemp)

testcase_cleanup() { rm -f "$ERR"; }

ffs -d --no-output -m "$MNT" ../json/object.json &
PID=$!
"$WAITFOR" mount "$MNT"
chown :nobody "$MNT"/name 2>$ERR >&2 && fail "chgrp1: $(cat $ERR)"
[ -s "$ERR" ] || fail "chgrp1 error: $(cat $ERR)"
chown :$(groups | cut -d' ' -f 1) "$MNT"/name 2>$ERR >&2 || fail "chgrp2: $(cat $ERR)"
[ -s "$ERR" ] && fail "chgrp2 error: $(cat $ERR)"
chown $(whoami) "$MNT"/name 2>$ERR >&2 || fail chown
[ -s "$ERR" ] && fail "chown error: $(cat $ERR)"
"$WAITFOR" umount "$MNT" || fail unmount1
"$WAITFOR" exit $PID || fail process1

rmdir "$MNT" || fail mount
rm "$ERR"
