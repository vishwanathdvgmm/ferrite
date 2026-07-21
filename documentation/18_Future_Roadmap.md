# Future Roadmap

This document outlines the strategic architectural roadmap for the Ferrite compiler and language. It maps the transition from the current stable release (v2.4.1) toward the next major milestones (v3.0 and v4.0), focusing on resolving technical debt, expanding the machine learning backend, and achieving scale.

## 1. Long-Term Vision

The ultimate goal of the Ferrite Project is to eliminate the "Two-Language Problem" in AI engineering by providing a single, unified language that possesses the ergonomic fluidity of Python and the bare-metal performance of C++/CUDA.

The Ferrite compiler must eventually be capable of self-hosting (compiling itself) and natively targeting advanced accelerator hardware (GPUs/TPUs) without relying on massive, opaque C++ frameworks like PyTorch or TensorFlow.

## 2. Upcoming Features (v3.0: The Ecosystem Update)

The v2.x series successfully stabilized the core language semantics (strict typing, ML blocks, module namespaces). The v3.0 series will focus entirely on ecosystem scalability.

### 2.1 The Ferrite Package Manager (`fpm`)

- **Goal:** Provide a native, decentralized package manager integrated directly into the compiler CLI.
- **Architecture:** `ferrite get <url>` will clone repositories, verify cryptographic hashes, and cache dependencies globally in `~/.ferrite/registry`.
- **Language Impact:** The Import Resolver (`src/imports/mod.rs`) will be upgraded to resolve module paths from the local cache rather than strictly relative filesystem paths.

### 2.2 Standard Library Expansion

- **Goal:** Expand the embedded standard library to include native HTTP handling, JSON parsing, and advanced Linear Algebra primitives.
- **Architecture:** Move beyond pure-Ferrite implementations and expose more native OS-level bindings (`extern "C"`) through the `TypeEnv` to ensure the stdlib runs at maximum speed.

## 3. Compiler Architecture Improvements

### 3.1 MLIR Backend Integration

- **Goal:** Move beyond pure CPU execution and target GPUs natively.
- **Architecture:** While LLVM is excellent for CPU compilation, it struggles to optimize massively parallel, multi-dimensional tensor operations. Ferrite will integrate **MLIR** (Multi-Level Intermediate Representation). By lowering the AST to MLIR's `linalg` or `tensor` dialects instead of pure LLVM IR, Ferrite can leverage specialized passes to compile directly to NVIDIA PTX (CUDA) and AMD ROCm.

### 3.2 Incremental Compilation & Language Server (LSP)

- **Goal:** Provide a world-class, instantaneous IDE experience.
- **Architecture:** Rewrite the `DiagnosticBag` and the `SemanticAnalyzer` to support Incremental Compilation using a query-based framework (similar to Rust's `salsa`). When a developer types a character in an IDE, the compiler must only re-parse and re-typecheck the specific function being edited, rather than re-evaluating the entire file from scratch.

## 4. Language Improvements

### 4.1 Algebraic Data Type (Enum) Methods

- **Goal:** Allow enums to have `fun` methods directly attached to them, similar to `group` (struct) methods.
- **Impact:** Reduces boilerplate when querying the state of complex ADTs.

### 4.2 Explicit Lifetimes vs ARC

- **Goal:** Eliminate the runtime overhead of Automatic Reference Counting (ARC) for performance-critical ML memory.
- **Architecture:** Introduce a borrow-checker and explicit lifetime annotations (e.g., `'a`). This is a massive, breaking architectural shift and is currently slated for research in the v4.0 horizon. It will require overhauling the `TypeEnv` to track variable ownership and borrow states.

## 5. Performance Roadmap

Resolving the structural bottlenecks identified in `14_Performance.md` is critical to achieving the goal of compiling 1,000,000 lines of code per second.

### Phase 1: String Interning (Target: v2.5)

- Replace all heap-allocated `String` identifiers in the AST with a `Symbol` (`u32`).
- Implement a global `StringInterner` struct passed alongside the `DiagnosticBag`.
- Transition the `TypeEnv` from `HashMap<String, Type>` to `HashMap<Symbol, Type>`.

### Phase 2: Fast Hashing (Target: v2.5)

- Strip out Rust's default `SipHash` algorithm.
- Integrate `rustc-hash` (FxHash) for all internal compiler Hash Maps to maximize symbol lookup speed during Semantic Analysis.

### Phase 3: Arena Allocation (Target: v2.6)

- Remove all `Box<T>` wrappers from `src/ast/mod.rs`.
- Integrate `bumpalo` to allocate the entire AST in contiguous memory blocks.
- **Expected Result:** A 30-50% reduction in compilation time due to massive improvements in CPU cache hit rates and the elimination of recursive heap deallocations.
