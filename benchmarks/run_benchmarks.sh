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

export LLVM_SYS_150_PREFIX="F:/Applications/LLVM_15/LLVM"
export PATH="/f/Applications/LLVM_15/LLVM/bin:$PATH"

# Build Ferrite in release mode
echo "Building Ferrite (Release mode)..."
cargo build --release --features llvm
export PATH="/f/Softwares/lua-5.5.0/lua-5.5.0/src:$PATH"
FERRITE_CMD="./target/release/ferrite.exe"

# Build Ferrite native benchmarks
echo "Building Ferrite native benchmarks via LLVM..."
for cat_dir in benchmarks/*; do
    if [ -d "$cat_dir" ]; then
        for test_dir in "$cat_dir"/*; do
            if [ -d "$test_dir" ]; then
                test=$(basename "$test_dir")
                if [ -f "$test_dir/${test}.fe" ]; then
                    $FERRITE_CMD compile "$test_dir/${test}.fe" > /dev/null 2>&1
                    clang -O3 "$test_dir/${test}.ll" "src/runtime/c/ferrite_rt.c" -o "$test_dir/${test}_ferrite"
                fi
            fi
        done
    fi
done

# Build Rust and Go benchmarks
echo "Building Rust and Go benchmarks..."
for cat_dir in benchmarks/*; do
    if [ -d "$cat_dir" ]; then
        for test_dir in "$cat_dir"/*; do
            if [ -d "$test_dir" ]; then
                test=$(basename "$test_dir")
                if [ -f "$test_dir/${test}.rs" ]; then
                    rustc -O "$test_dir/${test}.rs" -o "$test_dir/${test}_rs"
                fi
                if [ -f "$test_dir/${test}.go" ]; then
                    go build -o "$test_dir/${test}_go" "$test_dir/${test}.go"
                fi
            fi
        done
    fi
done

run_benchmark() {
    local cmd="$1"
    shift 1
    local args=("$@")

    if [[ "$cmd" != ./* ]] && ! command -v "$cmd" &> /dev/null; then
        printf "%-10s" "N/A"
        return
    fi

    local start_time
    start_time=$(python -c 'import time; print(int(time.time() * 1000))')
    
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

printf "%-20s | %-10s | %-10s | %-10s | %-10s | %-10s | %-10s\n" "Benchmark" "Ferrite" "Python" "Lua" "Node.js" "Rust" "Go"
echo "-----------------------------------------------------------------------------------------------"

for cat_dir in benchmarks/*; do
    if [ -d "$cat_dir" ]; then
        for test_dir in "$cat_dir"/*; do
            if [ -d "$test_dir" ]; then
                test=$(basename "$test_dir")
                printf "%-20s | " "$test"
                
                run_benchmark "$test_dir/${test}_ferrite"
                printf " | "
                
                run_benchmark "python" "$test_dir/$test.py"
                printf " | "
                
                run_benchmark "lua" "$test_dir/$test.lua"
                printf " | "
                
                run_benchmark "node" "$test_dir/$test.js"
                printf " | "
                
                run_benchmark "$test_dir/${test}_rs"
                printf " | "
                
                run_benchmark "$test_dir/${test}_go"
                printf "\n"
            fi
        done
    fi
done

echo ""
echo "========================================="

