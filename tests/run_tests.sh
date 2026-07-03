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
echo "  Ferrite v2.3.1 — Compiler Verification Suite"
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
    # Check if it parses and typechecks
    output=$("$FERRITE" check "$test_file" 2>&1) && exit_code=0 || exit_code=$?
    
    if [ "$exit_code" -eq 0 ]; then
        # Also ensure it runs without crashing. Provide mock stdin for tests like pass_11_builtins
        run_output=$(echo "test_input" | "$FERRITE" run "$test_file" 2>&1) && run_code=0 || run_code=$?
        if [ "$run_code" -eq 0 ]; then
            echo "  ✅ PASS  $test_name"
            PASS=$((PASS + 1))
        else
            echo "  ❌ FAIL  $test_name (typecheck ok, but execution failed with exit $run_code)"
            echo "          Output: $run_output"
            FAIL=$((FAIL + 1))
            ERRORS+=("$test_name: execution failed")
        fi
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
    ["fail_14_missing_trait_method"]="Missing method"
    ["fail_15_trait_not_found"]="is not defined"
    ["fail_16_trait_bound_violated"]="does not implement trait"
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

# ── RUNTIME FAIL Tests ───────────────────────────────────────────
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  RUNTIME FAIL TESTS (must fail with exit code 1 during run)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

declare -A EXPECTED_RUNTIME_ERRORS=(
    ["runtime_fail_01_div_by_zero"]="Division by zero"
    ["runtime_fail_02_negative_index"]="Negative index"
    ["runtime_fail_03_index_bounds"]="out of bounds"
)

for test_file in "$TESTS_DIR"/runtime_fail_*.fe; do
    if [ ! -f "$test_file" ]; then break; fi
    test_name=$(basename "$test_file" .fe)
    
    # Run the file (not just check)
    output=$("$FERRITE" run "$test_file" 2>&1) && exit_code=0 || exit_code=$?
    expected_err="${EXPECTED_RUNTIME_ERRORS[$test_name]:-Runtime Error}"
    
    if [ "$exit_code" -ne 0 ]; then
        if echo "$output" | grep -qi "$expected_err"; then
            echo "  ✅ PASS  $test_name  (correctly failed at runtime with: \"$expected_err\")"
            PASS=$((PASS + 1))
        else
            echo "  ⚠️  PARTIAL  $test_name  (failed, but missing expected error: \"$expected_err\")"
            echo "          Actual output: $output"
            FAIL=$((FAIL + 1))
            ERRORS+=("$test_name: runtime rejected but wrong error message")
        fi
    else
        echo "  ❌ FAIL  $test_name  (expected runtime rejection, but it ran successfully!)"
        FAIL=$((FAIL + 1))
        ERRORS+=("$test_name: expected to fail at runtime but passed")
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
    echo "  🎉 ALL $TOTAL TESTS PASSED — Ferrite v2.3.1 is verified!"
    echo ""
    exit 0
fi
