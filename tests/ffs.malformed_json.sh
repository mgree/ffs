#!/bin/sh

. ./fail.def

MALFORMED=$(mktemp --suffix=.json)

# Create a malformed JSON file
cat > "$MALFORMED" <<'EOF'
{
  "name": "test",
  "invalid": ,
  "value": 123
}
EOF

# Try to mount - should fail with error message (not panic)
OUTPUT=$(ffs -m /tmp/test_mount "$MALFORMED" 2>&1)
EC=$?

# Should exit with error status
[ $EC -ne 0 ] || fail "expected non-zero exit code"

# Error message should mention "parse" or "JSON" and not say "panic"
echo "$OUTPUT" | grep -qi "json" || fail "error should mention JSON"
echo "$OUTPUT" | grep -qi "panic" && fail "should not panic"

rm "$MALFORMED"
