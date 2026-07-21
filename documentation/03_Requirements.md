# Requirements

This document outlines the strict engineering requirements that govern the architecture, implementation, and ongoing evolution of the Ferrite compiler and runtime. These requirements dictate the boundaries of what the compiler must do, what it must never do, and the performance baseline it must maintain.

## 1. Functional Requirements

Functional requirements define the core capabilities the Ferrite compiler and runtime must expose to the programmer.

- **F-REQ-01: Native Tensor Representation.** The compiler must support `Tensor<Type, Shape>` as a first-class primitive in the Abstract Syntax Tree (AST). The type checker must be capable of verifying scalar operations, matrix multiplications, and tensor broadcasts at compile-time.
- **F-REQ-02: Compile-Time Shape Inference.** The semantic analyzer must resolve and enforce tensor dimensionalities statically. Unresolved generic shape parameters in functions (e.g., `fun conv2d<N, C, H, W>`) must be instantiated and validated against concrete arguments during call resolution.
- **F-REQ-03: Contextual ML Blocks.** The parser and runtime must support contextual `train` and `infer` blocks. Code executed within a `train` block must trigger the generation/retention of a computation graph for automatic differentiation. Code in an `infer` block must statically elide graph generation.
- **F-REQ-04: Module and Namespace Isolation.** The language must provide strict namespace encapsulation. Top-level declarations are private to their module by default and must be explicitly exported using the `pub` keyword. Cross-file symbol resolution must support aliased (`import as`) and selective (`from take`) imports.
- **F-REQ-05: Structured Concurrency.** The language must provide `spawn` for lightweight task creation, `await` for synchronization, and `select` for non-blocking task multiplexing.
- **F-REQ-06: Zero Implicit Coercion.** The compiler must explicitly reject any operation that attempts to combine or assign variables of different underlying numeric types without an explicit cast.

## 2. Non-Functional Requirements

Non-functional requirements dictate the architectural quality, maintainability, and operational characteristics of the compiler infrastructure.

- **NF-REQ-01: Deterministic Memory Management.** The runtime must manage memory without a non-deterministic stop-the-world garbage collector. All allocations and deallocations must be deterministically tied to scope exits, ownership semantics, or Automatic Reference Counting (ARC).
- **NF-REQ-02: Single-Binary Deployment.** A compiled Ferrite application must yield a statically linked native executable (or dynamically linked only to standard OS libraries like `libc` / `libm`), requiring no heavy external runtime dependencies (such as a JVM or Python runtime) to execute on a target machine.
- **NF-REQ-03: Meaningful Diagnostics.** The compiler must not halt on the first syntax or type error. The `DiagnosticBag` architecture must accumulate errors across lexical, parsing, and semantic phases to provide the developer with comprehensive, multi-point error reporting in a single compilation pass.
- **NF-REQ-04: Modular Compiler Architecture.** The compiler pipeline (Lexer → Parser → Semantic Analyzer → CodeGen/Interpreter) must remain strictly decoupled. The AST output from the Parser must be entirely independent of the backend lowering logic.

## 3. Performance Requirements

Ferrite is designed for high-performance computing; therefore, its execution speed must be strictly governed.

- **P-REQ-01: Execution Speed.** Native binaries compiled via the LLVM backend must execute numerical benchmarks (e.g., dense matrix multiplication) within 5% of the execution time of an equivalently optimized C++ implementation.
- **P-REQ-02: Compilation Speed.** The compiler front-end (parsing and semantic analysis) must be capable of processing a minimum of 100,000 lines of code per second on modern hardware (e.g., M-series Apple Silicon or AMD Ryzen 7000 series) to ensure instantaneous IDE feedback (Language Server).
- **P-REQ-03: Zero-Overhead Abstractions.** Generics, Traits, and Group (Struct) methods must be fully monomorphized and resolved at compile-time. There must be no dynamic dispatch overhead (v-tables) unless explicitly requested via dynamic interface types.
- **P-REQ-04: Memory Footprint.** The base memory footprint of a compiled, idle Ferrite binary must not exceed 5 MB on a 64-bit architecture.

## 4. Security Requirements

- **S-REQ-01: Strict Bounds Checking.** By default, all tensor and array accesses must be bounds-checked at runtime to prevent buffer overflows and arbitrary memory execution. Bounds checks may only be elided via an explicit `unsafe` block or if the compiler can mathematically prove safety at compile-time.
- **S-REQ-02: Memory Safety (Dangling Pointers).** The compiler must guarantee the absence of dangling pointers and use-after-free vulnerabilities in safe Ferrite code through strict scope and lifetime validation.

## 5. Portability Requirements

- **PO-REQ-01: Cross-Platform Compilation.** The compiler must run on, and generate native binaries for, `x86_64` and `aarch64` architectures running Windows, macOS, or Linux.
- **PO-REQ-02: Target Agnosticism.** The intermediate representation (IR) generated by the semantic analyzer must be entirely target-agnostic. All hardware-specific lowering (e.g., vectorization, GPU offloading) must be strictly isolated to the backend codegen module.

## 6. Future Requirements

While not implemented in the current baseline, the architecture must remain extensible to accommodate the following future capabilities without requiring a fundamental rewrite:

- **FU-REQ-01: GPU/TPU Backend.** The compiler must eventually support a backend that lowers Ferrite AST directly to PTX (CUDA), MLIR, or SPIR-V for execution on specialized accelerator hardware.
- **FU-REQ-02: Distributed Training.** The concurrency model (`spawn` / `select`) must be architecturally extensible to span across network boundaries for distributed computing.
- **FU-REQ-03: Package Manager.** The module resolution system must be designed to eventually integrate with a centralized or decentralized package management system (v3.0 goal).
