#!/usr/bin/env bash
# check-sister-first.sh — detect direct local operations that should go through SisterGateway
#
# Run before push to catch sister-first violations.
# Exit code = number of violations found.

set -euo pipefail

VIOLATIONS=0
COGNITIVE_SRC="crates/hydra-native-cognitive/src"

echo "=== Sister-First Enforcement Check ==="

# 1. Direct fs::read_to_string in cognitive code (should use gateway.read_file)
HITS=$(grep -rn "fs::read_to_string\|std::fs::read" "$COGNITIVE_SRC/" \
  --include="*.rs" \
  | grep -v gateway | grep -v test | grep -v fallback | grep -v "mod.rs" || true)
if [ -n "$HITS" ]; then
  echo "WARNING: Direct fs::read found outside gateway — consider gateway.read_file()"
  echo "$HITS" | head -5
  echo ""
fi

# 2. Direct Command::new("find") (should use gateway.find_file)
HITS=$(grep -rn 'Command::new("find")' "$COGNITIVE_SRC/" \
  --include="*.rs" \
  | grep -v gateway | grep -v test || true)
if [ -n "$HITS" ]; then
  echo "WARNING: Direct find command found — consider gateway.find_file()"
  echo "$HITS" | head -5
  echo ""
fi

# 3. Direct Command::new("grep") (should use gateway.code_search)
HITS=$(grep -rn 'Command::new("grep")\|Command::new("rg")' "$COGNITIVE_SRC/" \
  --include="*.rs" \
  | grep -v gateway | grep -v test || true)
if [ -n "$HITS" ]; then
  echo "WARNING: Direct grep/rg command found — consider gateway.code_search()"
  echo "$HITS" | head -5
  echo ""
fi

# 4. Check gateway.rs and gateway_helpers.rs exist
if [ ! -f "$COGNITIVE_SRC/sisters/gateway.rs" ]; then
  echo "VIOLATION: gateway.rs does not exist"
  VIOLATIONS=$((VIOLATIONS + 1))
fi
if [ ! -f "$COGNITIVE_SRC/sisters/gateway_helpers.rs" ]; then
  echo "VIOLATION: gateway_helpers.rs does not exist"
  VIOLATIONS=$((VIOLATIONS + 1))
fi

# 5. Check gateway is exported from mod.rs
if ! grep -q "pub mod gateway;" "$COGNITIVE_SRC/sisters/mod.rs" 2>/dev/null; then
  echo "VIOLATION: gateway not exported from sisters/mod.rs"
  VIOLATIONS=$((VIOLATIONS + 1))
fi

echo ""
if [ "$VIOLATIONS" -eq 0 ]; then
  echo "Sister-first check: PASS (gateway exists and is wired)"
else
  echo "Sister-first violations: $VIOLATIONS"
fi

exit "$VIOLATIONS"
