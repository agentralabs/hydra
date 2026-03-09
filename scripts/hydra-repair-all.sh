#!/bin/bash
# hydra-repair-all.sh — Run ALL repair specs in priority order.
#
# Stops on first failure. Resume from last failure with --resume.
#
# Usage:
#   ./scripts/hydra-repair-all.sh              # Run all from start
#   ./scripts/hydra-repair-all.sh --resume     # Resume from last failure
#   ./scripts/hydra-repair-all.sh --check-only # Only run acceptance checks, no repair
#   ./scripts/hydra-repair-all.sh --status     # Show status of all specs

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPEC_DIR="$REPO_ROOT/repair-specs"
STATE_FILE="$REPO_ROOT/.hydra-repair-state"
SELF_REPAIR="$REPO_ROOT/scripts/hydra-self-repair.sh"
LOG_DIR="/tmp/hydra-repair"

mkdir -p "$LOG_DIR"

MODE="${1:-run}"

# ── Status: show all specs and their check status ──
if [ "$MODE" = "--status" ]; then
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  HYDRA SELF-REPAIR STATUS"
    echo "═══════════════════════════════════════════════════════════"
    echo ""

    for spec in $(find "$SPEC_DIR" -name '*.json' | sort); do
        TASK=$(python3 -c "import json; print(json.load(open('$spec'))['task'])" 2>/dev/null || echo "?")
        BASENAME=$(basename "$spec")

        # Quick check: run acceptance checks silently
        if python3 -c "
import json, subprocess, sys
spec = json.load(open('$spec'))
checks = spec.get('acceptance_checks', [])
passed = 0
for c in checks:
    r = subprocess.run(c['check'], shell=True, capture_output=True, text=True, cwd='$REPO_ROOT', timeout=30)
    out = (r.stdout + r.stderr).strip()
    ok = False
    if 'expect' in c:
        if c['expect'] == 'found': ok = r.returncode == 0 and len(r.stdout.strip()) > 0
        elif c['expect'] == 'not_found': ok = r.returncode != 0 or len(r.stdout.strip()) == 0
        else: ok = str(c['expect']) in out
    if 'expect_minimum' in c:
        nums = [int(w) for w in out.split() if w.isdigit()]
        ok = nums[-1] >= c['expect_minimum'] if nums else False
    if ok: passed += 1
print(f'{passed}/{len(checks)}')
sys.exit(0 if passed == len(checks) else 1)
" 2>/dev/null; then
            STATUS="\033[32m✅ PASS\033[0m"
        else
            RATIO=$(python3 -c "
import json, subprocess, sys
spec = json.load(open('$spec'))
checks = spec.get('acceptance_checks', [])
passed = 0
for c in checks:
    r = subprocess.run(c['check'], shell=True, capture_output=True, text=True, cwd='$REPO_ROOT', timeout=30)
    out = (r.stdout + r.stderr).strip()
    ok = False
    if 'expect' in c:
        if c['expect'] == 'found': ok = r.returncode == 0 and len(r.stdout.strip()) > 0
        elif c['expect'] == 'not_found': ok = r.returncode != 0 or len(r.stdout.strip()) == 0
        else: ok = str(c['expect']) in out
    if 'expect_minimum' in c:
        nums = [int(w) for w in out.split() if w.isdigit()]
        ok = nums[-1] >= c['expect_minimum'] if nums else False
    if ok: passed += 1
print(f'{passed}/{len(checks)}')
" 2>/dev/null || echo "?/?")
            STATUS="\033[33m⚠️  $RATIO\033[0m"
        fi

        printf "  %s  %-45s %s\n" "$STATUS" "$BASENAME" "$TASK"
    done

    echo ""
    exit 0
fi

# ── Check-only mode: run all checks without repair ──
if [ "$MODE" = "--check-only" ]; then
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  HYDRA SELF-REPAIR: CHECK-ONLY MODE"
    echo "═══════════════════════════════════════════════════════════"

    TOTAL=0
    ALL_PASS=0
    PARTIAL=0

    for spec in $(find "$SPEC_DIR" -name '*.json' | sort); do
        TOTAL=$((TOTAL + 1))
        TASK=$(python3 -c "import json; print(json.load(open('$spec'))['task'])" 2>/dev/null || echo "?")
        echo ""
        echo "── $(basename "$spec"): $TASK ──"
        if bash "$SELF_REPAIR" "$spec" 2>/dev/null | head -1 | grep -q "PASS"; then
            ALL_PASS=$((ALL_PASS + 1))
        else
            PARTIAL=$((PARTIAL + 1))
        fi
    done

    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  Results: $ALL_PASS/$TOTAL fully passing, $PARTIAL need repair"
    echo "═══════════════════════════════════════════════════════════"
    exit 0
fi

# ── Resume: read last failure point ──
START_FROM="000"
if [ "$MODE" = "--resume" ] && [ -f "$STATE_FILE" ]; then
    START_FROM=$(cat "$STATE_FILE")
    echo "Resuming from spec $START_FROM"
fi

# ── Main repair loop ──
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  HYDRA SELF-REPAIR: MASTER RUNNER"
echo "═══════════════════════════════════════════════════════════"

for spec in $(find "$SPEC_DIR" -name '*.json' | sort); do
    SPEC_NUM=$(basename "$spec" | cut -d'-' -f1)

    # Skip if before resume point
    if [ "$SPEC_NUM" \< "$START_FROM" ]; then
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    TOTAL=$((TOTAL + 1))
    TASK=$(python3 -c "import json; print(json.load(open('$spec'))['task'])" 2>/dev/null || echo "?")

    echo ""
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║  REPAIR $SPEC_NUM: $TASK"
    echo "╚═══════════════════════════════════════════════════════════╝"

    if bash "$SELF_REPAIR" "$spec"; then
        PASSED=$((PASSED + 1))
        echo "  ✅ $(basename "$spec") — COMPLETE"
    else
        FAILED=$((FAILED + 1))
        echo "  ❌ $(basename "$spec") — FAILED after max iterations"
        echo "$SPEC_NUM" > "$STATE_FILE"
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  REPAIR PAUSED at spec $SPEC_NUM"
        echo "  Passed: $PASSED / $TOTAL"
        echo "  Failed: $FAILED"
        echo "  Skipped: $SKIPPED"
        echo "  Resume with: ./scripts/hydra-repair-all.sh --resume"
        echo "═══════════════════════════════════════════════════════════"
        exit 1
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  ALL REPAIRS COMPLETE"
echo "  Total: $TOTAL  Passed: $PASSED  Failed: $FAILED  Skipped: $SKIPPED"
echo "═══════════════════════════════════════════════════════════"
rm -f "$STATE_FILE"
