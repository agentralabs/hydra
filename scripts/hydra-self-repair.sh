#!/bin/bash
# hydra-self-repair.sh — Hydra's bootstrap self-repair loop.
#
# Reads a JSON spec file with acceptance checks, invokes Claude Code to
# implement the fix, validates with cargo test + grep checks, and loops
# until ALL checks pass or max iterations reached.
#
# Usage: ./scripts/hydra-self-repair.sh repair-specs/001-wire-memory-learn.json
#
# Requirements: bash, python3 (stdlib only — no pip packages), cargo, claude CLI

set -euo pipefail

SPEC_FILE="${1:-}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="/tmp/hydra-repair"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

if [ -z "$SPEC_FILE" ]; then
    echo "Usage: ./scripts/hydra-self-repair.sh <spec-file.json>"
    exit 1
fi

if [ ! -f "$SPEC_FILE" ]; then
    echo "Error: Spec file not found: $SPEC_FILE"
    exit 1
fi

mkdir -p "$LOG_DIR"

# ── Parse spec with Python3 (stdlib json only) ──
TASK=$(python3 -c "import json; print(json.load(open('$SPEC_FILE'))['task'])")
MAX_ITER=$(python3 -c "import json; print(json.load(open('$SPEC_FILE')).get('max_iterations', 5))")
PRIORITY=$(python3 -c "import json; print(json.load(open('$SPEC_FILE')).get('priority', 0))")

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  HYDRA SELF-REPAIR: $TASK"
echo "  Spec: $(basename "$SPEC_FILE")"
echo "  Priority: $PRIORITY  |  Max iterations: $MAX_ITER"
echo "═══════════════════════════════════════════════════════════"

# ── Run acceptance checks — returns 0 if ALL pass, 1 if any fail ──
run_acceptance_checks() {
    local log_file="${1:-/dev/null}"

    python3 << 'PYEOF' "$SPEC_FILE" "$REPO_ROOT" "$log_file"
import json, subprocess, sys, os

spec_file = sys.argv[1]
repo_root = sys.argv[2]
log_file  = sys.argv[3]

spec   = json.load(open(spec_file))
checks = spec.get("acceptance_checks", [])

all_pass  = True
failures  = []
results   = []

for check in checks:
    name = check["name"]
    cmd  = check["check"]

    try:
        result = subprocess.run(
            cmd, shell=True, capture_output=True, text=True,
            cwd=repo_root, timeout=120,
        )
        output = (result.stdout.strip() + " " + result.stderr.strip()).strip()
    except subprocess.TimeoutExpired:
        output = "TIMEOUT"
        result = type("R", (), {"returncode": 1, "stdout": "", "stderr": ""})()

    passed = False

    if "expect" in check:
        exp = check["expect"]
        if exp == "found":
            passed = result.returncode == 0 and len(result.stdout.strip()) > 0
        elif exp == "not_found":
            passed = result.returncode != 0 or len(result.stdout.strip()) == 0
        else:
            passed = str(exp) in output

    if "expect_minimum" in check:
        try:
            # Extract the last number from output
            nums = [int(w) for w in output.split() if w.isdigit()]
            value = nums[-1] if nums else 0
            passed = value >= check["expect_minimum"]
        except (ValueError, IndexError):
            passed = False

    if "expect_maximum" in check:
        try:
            nums = [int(w) for w in output.split() if w.isdigit()]
            value = nums[-1] if nums else 999999
            passed = value <= check["expect_maximum"]
        except (ValueError, IndexError):
            passed = False

    status = "\033[32m✅ PASS\033[0m" if passed else "\033[31m❌ FAIL\033[0m"
    print(f"  {status}: {name}")

    entry = {"name": name, "passed": passed, "output": output[:300]}
    results.append(entry)

    if not passed:
        all_pass = False
        failures.append(entry)

# Summary
if all_pass:
    print(f"\n  \033[32mALL {len(checks)} CHECKS PASSED ✅\033[0m")
else:
    print(f"\n  \033[31m{len(failures)}/{len(checks)} CHECK(S) FAILED ❌\033[0m")
    for f in failures:
        print(f"    → {f['name']}: {f['output'][:120]}")

# Write results to log file
if log_file != "/dev/null":
    with open(log_file, "w") as fp:
        json.dump({"all_pass": all_pass, "results": results, "failures": failures}, fp, indent=2)

sys.exit(0 if all_pass else 1)
PYEOF
    return $?
}

# ── Main repair loop ──
ITERATION=0

while [ "$ITERATION" -lt "$MAX_ITER" ]; do
    ITERATION=$((ITERATION + 1))
    ITER_LOG="$LOG_DIR/$(basename "$SPEC_FILE" .json)-iter${ITERATION}-${TIMESTAMP}.log"
    CHECK_LOG="$LOG_DIR/$(basename "$SPEC_FILE" .json)-checks-${ITERATION}.json"

    echo ""
    echo "───────────────────────────────────────────────"
    echo "  ITERATION $ITERATION of $MAX_ITER"
    echo "───────────────────────────────────────────────"

    # Run acceptance checks to see current state
    echo ""
    echo "── Pre-check: current state ──"
    if run_acceptance_checks "$CHECK_LOG"; then
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  ✅ ALL CHECKS ALREADY PASS — No repair needed!"
        echo "═══════════════════════════════════════════════════════════"
        exit 0
    fi

    echo ""
    echo "── Launching Claude Code for repair (iteration $ITERATION) ──"

    # Build the prompt for Claude Code
    if [ "$ITERATION" -eq 1 ]; then
        INSTRUCTIONS=$(python3 -c "import json; print(json.load(open('$SPEC_FILE'))['instructions_for_claude_code'])")
        PROMPT="$INSTRUCTIONS"
    else
        # On retry: include the failure log
        FAILURE_DETAIL=$(python3 -c "
import json, sys
try:
    data = json.load(open('$CHECK_LOG'))
    for f in data.get('failures', []):
        print(f'FAILED: {f[\"name\"]}')
        print(f'  Output: {f[\"output\"][:200]}')
        print()
except: pass
")
        INSTRUCTIONS=$(python3 -c "import json; print(json.load(open('$SPEC_FILE'))['instructions_for_claude_code'])")
        PROMPT="PREVIOUS ATTEMPT FAILED (iteration $((ITERATION - 1))). Here are the specific failures:

$FAILURE_DETAIL

Fix ALL failing checks. The acceptance criteria are non-negotiable.
Do not claim 'done' until all checks pass. Run cargo check and cargo test yourself.

Original instructions:
$INSTRUCTIONS"
    fi

    # Run Claude Code with the repair instructions
    echo "$PROMPT" | claude --dangerously-skip-permissions \
        --print \
        --output-format text \
        2>&1 | tee "$ITER_LOG"

    echo ""
    echo "── Claude Code completed iteration $ITERATION ──"
    echo "── Running acceptance checks ──"
    echo ""

    if run_acceptance_checks "$CHECK_LOG"; then
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  ✅ REPAIR COMPLETE — All checks pass"
        echo "  Task: $TASK"
        echo "  Iterations: $ITERATION"
        echo "  Log: $ITER_LOG"
        echo "═══════════════════════════════════════════════════════════"

        # Record result
        echo "{\"spec\": \"$(basename "$SPEC_FILE")\", \"task\": \"$TASK\", \"iterations\": $ITERATION, \"status\": \"success\", \"timestamp\": \"$TIMESTAMP\"}" \
            >> "$LOG_DIR/repair-history.jsonl"

        exit 0
    fi

    echo ""
    echo "  ❌ Checks still failing after iteration $ITERATION"

    if [ "$ITERATION" -eq "$MAX_ITER" ]; then
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  ❌ REPAIR FAILED after $MAX_ITER iterations"
        echo "  Task: $TASK"
        echo "  Escalating to human."
        echo ""
        echo "  Iteration logs: $LOG_DIR/$(basename "$SPEC_FILE" .json)-iter*"
        echo "  Check results:  $CHECK_LOG"
        echo ""
        echo "  Failing checks:"
        run_acceptance_checks /dev/null 2>&1 || true
        echo "═══════════════════════════════════════════════════════════"

        echo "{\"spec\": \"$(basename "$SPEC_FILE")\", \"task\": \"$TASK\", \"iterations\": $ITERATION, \"status\": \"failed\", \"timestamp\": \"$TIMESTAMP\"}" \
            >> "$LOG_DIR/repair-history.jsonl"

        exit 1
    fi
done
