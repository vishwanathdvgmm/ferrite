#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# Ferrite v2.0 — Rigorous Test Runner
# Tests both PASS cases (must exit 0) and FAIL cases (must exit 1)
# ─────────────────────────────────────────────────────────────────

set -euo pipefail

FERRITE="./target/debug/ferrite"
TESTS_DIR="./tests"
PASS=0
FAIL=0
ERRORS=()

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Ferrite v2.1 — Compiler Verification Suite"
echo "══════════════════════════════════════════════════════════════"
echo ""

# ── Build ────────────────────────────────────────────────────────
echo "🔨 Building compiler..."
cargo build 2>&1
echo ""

# ── PASS Tests ───────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  PASS TESTS (must succeed with exit code 0)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

for test_file in "$TESTS_DIR"/pass_*.fe; do
    test_name=$(basename "$test_file" .fe)
    output=$("$FERRITE" check "$test_file" 2>&1) && exit_code=0 || exit_code=$?
    
    if [ "$exit_code" -eq 0 ]; then
        echo "  ✅ PASS  $test_name"
        PASS=$((PASS + 1))
    else
        echo "  ❌ FAIL  $test_name  (expected exit 0, got $exit_code)"
        echo "          Output: $output"
        FAIL=$((FAIL + 1))
        ERRORS+=("$test_name: expected to pass but failed with exit $exit_code")
    fi
done

echo ""

# ── FAIL Tests ───────────────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  FAIL TESTS (must fail with exit code 1 + correct error)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Declare expected error substrings for each fail test
declare -A EXPECTED_ERRORS=(
    ["fail_01_type_mismatch"]="Type mismatch"
    ["fail_02_undefined_var"]="Undefined variable"
    ["fail_03_return_mismatch"]="Type mismatch"
    ["fail_04_non_bool_condition"]="Type mismatch"
    ["fail_05_stop_skip_outside"]="outside of a loop"
    ["fail_06_duplicate_var"]="already defined"
    ["fail_07_no_coercion"]="Type mismatch"
    ["fail_08_syntax_missing_semi"]="Expected ';'"
    ["fail_09_syntax_missing_brace"]="Expected '}'"
    ["fail_10_negate_string"]="Negation requires a numeric type"
    ["fail_11_tensor_bad_elem"]="Tensors can only contain"
    ["fail_12_logic_non_bool"]="Type mismatch"
    ["fail_13_call_args"]="Function expects"
)

for test_file in "$TESTS_DIR"/fail_*.fe; do
    test_name=$(basename "$test_file" .fe)
    output=$("$FERRITE" check "$test_file" 2>&1) && exit_code=0 || exit_code=$?
    expected_err="${EXPECTED_ERRORS[$test_name]:-error}"
    
    if [ "$exit_code" -ne 0 ]; then
        # Check that the proper error message substring is present
        if echo "$output" | grep -qi "$expected_err"; then
            echo "  ✅ PASS  $test_name  (correctly rejected with: \"$expected_err\")"
            PASS=$((PASS + 1))
        else
            echo "  ⚠️  PARTIAL  $test_name  (rejected, but missing expected error: \"$expected_err\")"
            echo "          Actual output: $output"
            FAIL=$((FAIL + 1))
            ERRORS+=("$test_name: rejected but wrong error message")
        fi
    else
        echo "  ❌ FAIL  $test_name  (expected rejection, but it passed!)"
        FAIL=$((FAIL + 1))
        ERRORS+=("$test_name: expected to fail but passed")
    fi
done

echo ""

# ── Summary ──────────────────────────────────────────────────────
TOTAL=$((PASS + FAIL))
echo "══════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS/$TOTAL passed"
echo "══════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "  Failures:"
    for err in "${ERRORS[@]}"; do
        echo "    • $err"
    done
    echo ""
    exit 1
else
    echo ""
    echo "  🎉 ALL $TOTAL TESTS PASSED — Ferrite v2.1 is verified!"
    echo ""
    exit 0
fi
