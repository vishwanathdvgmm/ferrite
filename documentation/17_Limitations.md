# Limitations and Technical Debt

This document catalogs the known architectural flaws, unsupported features, and accumulated technical debt within the Ferrite v2.4.1 codebase. It serves as a transparent risk assessment for engineers maintaining or extending the compiler.

## 1. Known Issues

### 1.1 Ghost Errors During Semantic Recovery

When the Semantic Analyzer encounters a fatal type mismatch, it logs the error to the `DiagnosticBag` and artificially injects a `Type::Any` fallback type to allow parsing to continue.
**Issue:** If a complex generic function fails to resolve its bounds, the fallback `Type::Any` can sometimes propagate into downstream traits, causing the analyzer to silently accept mathematically invalid operations further down the AST, or worse, emit confusing secondary errors that trace back to the original fallback type.

### 1.2 `inkwell` (LLVM) Build Fragility

Ferrite relies on the `inkwell` crate to bind to the LLVM 15 C++ API.
**Issue:** Compiling `inkwell` from source requires a highly specific local environment (LLVM 15 headers, specific versions of `clang` and `llvm-config`). On Windows, this frequently breaks due to MSVC linker clashes. This forces the `llvm` feature to remain gated (`#[cfg(feature = "llvm")]`), splitting the compiler into two distinct operational modes (Interpreter vs Native).

## 2. Unsupported Features

### 2.1 Explicit Lifetimes / Borrow Checker

Unlike Rust, Ferrite does not currently possess a Borrow Checker. To ensure memory safety without Garbage Collection, the LLVM backend and the AST Interpreter rely heavily on Automatic Reference Counting (ARC) for heap-allocated objects (Tensors, Groups, Closures).
**Limitation:** ARC incurs a slight runtime penalty (atomic increments/decrements) every time a complex variable crosses a function boundary. High-frequency ML loops can suffer from cache thrashing due to these atomic updates.

### 2.2 Standardized Package Management

As of v2.4.1, Ferrite supports local module resolution via relative file paths (`import "utils/math"`), but lacks a unified package manager.
**Limitation:** There is no mechanism to pull third-party Ferrite code from a remote registry (e.g., no `ferrite get` or `Cargo.toml` equivalent for Ferrite projects). External dependencies must be manually copy-pasted into the source tree.

### 2.3 GPU Offloading

While Ferrite has first-class `Tensor` types, the current LLVM backend lowers these entirely to CPU-bound SIMD instructions.
**Limitation:** There is currently no MLIR or PTX backend to compile `train` and `infer` blocks directly for execution on NVIDIA (CUDA) or AMD (ROCm) hardware.

## 3. Accumulated Technical Debt

### 3.1 AST Heap Fragmentation

As noted in `14_Performance.md`, the Parser allocates every node in the AST using `Box<T>`.
**Debt:** This scatters the syntax tree randomly across the host OS heap memory. The Semantic Analyzer must chase these fragmented pointers, defeating CPU pre-fetching and significantly hurting cache locality during deep type-checking passes.
**Required Refactor:** Transition the AST to an Arena Allocator (`bumpalo`).

### 3.2 String-Based Type Lookup

The `TypeEnv` currently keys its symbol tables using `String` (e.g., `HashMap<String, Type>`).
**Debt:** Every variable lookup requires hashing the string. In deeply nested scopes or massive files, the cumulative time spent computing SipHash for simple variables (`i`, `j`, `x`) becomes a measurable drag on the compiler's theoretical throughput.
**Required Refactor:** Implement a global String Interner. Replace all AST identifiers with a 32-bit `SymbolId` and key the `TypeEnv` using integers.

### 3.3 Semantic Analyzer File Complexity

The `src/semantic/mod.rs` file handles Pass 1 (Global Registration), Pass 1.5 (Module Resolution), and Pass 2 (Expression Checking).
**Debt:** The file has grown excessively large and complex. The logic for verifying trait bounds (`where N: shape`) is tightly entangled with the logic for block scoping.
**Required Refactor:** Split the Semantic Analyzer into discrete sub-modules (`hoist.rs`, `resolve.rs`, `typecheck.rs`, `unify.rs`) communicating via the `TypeEnv`.

## 4. Architectural Risks

### 4.1 Ecosystem Fragmentation

Because the `ferrite compile` (LLVM) command is difficult to build on Windows, many users rely exclusively on the `ferrite run` (Interpreter) command.
**Risk:** If a bug in the AST Interpreter causes it to evaluate an expression differently than the LLVM backend, developers will unknowingly write code that works locally but fails or computes incorrect mathematics when compiled for production. The test suite must rigorously enforce absolute parity between the two execution engines.
