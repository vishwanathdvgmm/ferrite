#!/usr/bin/env bash
# Run Benchmarks

set -euo pipefail

echo "========================================="
echo "       Ferrite Benchmark Suite           "
echo "========================================="
echo ""

# Ensure we're in the right directory
if [ ! -d "./benchmarks" ]; then
    echo "Please run this script from the project root."
    exit 1
fi

# Build Ferrite in release mode
echo "Building Ferrite (Release mode)..."
cargo build --release
FERRITE_CMD="./target/release/ferrite"

# Build Rust benchmarks
echo "Building Rust benchmarks..."
rustc -O ./benchmarks/fibonacci.rs -o ./benchmarks/fibonacci_rs
rustc -O ./benchmarks/loop_sum.rs -o ./benchmarks/loop_sum_rs
rustc -O ./benchmarks/string_concat.rs -o ./benchmarks/string_concat_rs

# Build Go benchmarks
echo "Building Go benchmarks..."
go build -o ./benchmarks/fibonacci_go ./benchmarks/fibonacci.go
go build -o ./benchmarks/loop_sum_go ./benchmarks/loop_sum.go
go build -o ./benchmarks/string_concat_go ./benchmarks/string_concat.go

run_benchmark() {
    local name="$1"
    local cmd="$2"
    shift 2
    local args=("$@")

    # If it's not a path, check if it's available
    if [[ "$cmd" != ./* ]] && ! command -v "$cmd" &> /dev/null; then
        printf "%-10s" "N/A"
        return
    fi

    # Start time in milliseconds (cross-platform compatible way if possible)
    # Using python to get high resolution time since we already require python for benchmarks
    local start_time
    start_time=$(python -c 'import time; print(int(time.time() * 1000))')
    
    # Run command
    if "$cmd" "${args[@]}" > /dev/null 2>&1; then
        local end_time
        end_time=$(python -c 'import time; print(int(time.time() * 1000))')
        local diff=$((end_time - start_time))
        printf "%-10s" "${diff} ms"
    else
        printf "%-10s" "Failed"
    fi
}

echo ""
echo "Running benchmarks..."
echo ""

TESTS=("fibonacci" "loop_sum" "string_concat")

# Print Header
printf "%-20s | %-10s | %-10s | %-10s | %-10s | %-10s | %-10s\n" "Benchmark" "Ferrite" "Python" "Lua" "Node.js" "Rust" "Go"
echo "-----------------------------------------------------------------------------------------------"

for test in "${TESTS[@]}"; do
    printf "%-20s | " "$test"
    
    run_benchmark "$test" "$FERRITE_CMD" "run" "./benchmarks/$test.fe"
    printf " | "
    
    run_benchmark "$test" "python" "./benchmarks/$test.py"
    printf " | "
    
    run_benchmark "$test" "lua" "./benchmarks/$test.lua"
    printf " | "
    
    run_benchmark "$test" "node" "./benchmarks/$test.js"
    printf " | "
    
    run_benchmark "$test" "./benchmarks/${test}_rs"
    printf " | "
    
    run_benchmark "$test" "./benchmarks/${test}_go"
    printf "\n"
done

echo ""
echo "========================================="
