# Problem Statement

## 1. Why Ferrite Exists

Ferrite was created to solve the "Two-Language Problem" in modern machine learning and high-performance numerical computing. Currently, developing and deploying ML architectures requires context-switching between two entirely different paradigms: a high-level, dynamically typed language (usually Python) for prototyping, and a low-level, statically typed language (C/C++/CUDA) for execution and hardware acceleration.

This dichotomy forces engineering teams to maintain dual cognitive models and often requires rewriting research code for production. Ferrite exists to unify these two phases. It provides the syntactical ergonomics and rapid iteration speed of Python while enforcing the memory safety, strict determinism, and raw performance of systems languages like Rust and C++.

## 2. Existing Problems in the Ecosystem

The architecture of Ferrite is a direct response to several critical flaws in the current ML engineering landscape:

### 2.1 The Python Overhead and GIL

Python's dynamic nature and Global Interpreter Lock (GIL) inherently cripple high-throughput, multi-threaded numerical processing. While libraries like PyTorch and TensorFlow bypass the GIL by dropping down to C++ bindings, this abstraction leaks heavily when developers attempt to write custom data loaders, complex non-standard training loops, or multi-node coordination logic.

### 2.2 Silent Tensor Dimensionality Failures

In standard dynamic ML frameworks, tensor shapes are resolved at runtime. A matrix multiplication between a `(64, 128)` tensor and a `(64, 256)` tensor will crash deep within a training loop, often after hours of execution. There is no static guarantee that a neural network's architecture is dimensionally sound before execution begins.

### 2.3 Opacity of FFI Boundaries

When a dynamic language calls out to a C++ ML backend, the Foreign Function Interface (FFI) boundary creates an optimization black box. The host language compiler cannot look inside the C++ execution graph to optimize memory allocations, unroll loops, or fuse operations across the boundary.

### 2.4 Lack of First-Class ML Primitives

Mainstream systems programming languages (Rust, C++, Go) were not designed with machine learning in mind. Consequently, concepts like automatic differentiation, multi-dimensional array slicing, and inference-mode vs. training-mode memory semantics are bolted on via massive, complex library ecosystems rather than being natively understood by the compiler.

## 3. Motivation

The motivation behind Ferrite is to build a compiler that fundamentally understands machine learning.

If the compiler natively understands what a `Tensor` is, it can aggressively optimize memory alignment for SIMD operations. If the compiler natively understands the difference between a `train` block and an `infer` block, it can automatically discard the computation graph (gradient tape) during inference without requiring the programmer to manually toggle `.no_grad()` contexts.

By pushing ML semantics into the language AST and Type System, Ferrite eliminates entire categories of boilerplate and runtime errors.

## 4. Constraints

Designing a language to solve these problems introduces severe engineering constraints:

### 4.1 AOT Compilation Requirement

To achieve C-level performance and enable deployment on edge devices, Ferrite must be an Ahead-Of-Time (AOT) compiled language targeting LLVM (or equivalent backends), rather than relying on a heavy runtime VM or JIT compiler.

### 4.2 Deterministic Memory Management

Garbage collection (GC) pauses are unacceptable in low-latency inference systems (e.g., autonomous driving, high-frequency trading). Ferrite must employ deterministic memory management (Automatic Reference Counting or Ownership) to ensure predictable execution latency.

### 4.3 Static Type Rigidity

To guarantee tensor shapes at compile-time, the type system must be rigid enough to support dependent-like shape checking (e.g., `Tensor<float, (N, M)>`), yet ergonomic enough that developers aren't buried in verbose type annotations. Zero implicit coercion is a hard constraint to prevent silent precision loss.

## 5. Success Criteria

Ferrite's architecture will be considered successful if it achieves the following:

1. **Static Shape Verification:** 100% of tensor dimensionality mismatches and out-of-bounds structural errors are caught during the semantic analysis phase (compile-time) before a single byte of executable code is generated.
2. **Zero-Overhead Abstractions:** Abstracting ML operations into generic functions or struct methods incurs zero runtime cost compared to hand-written C code, verified via LLVM IR inspection.
3. **Seamless Concurrency:** Developers can spawn thousands of concurrent data-ingestion tasks using the native `spawn` and `select` primitives without manual thread management or race conditions.
4. **Standalone Deployability:** A compiled Ferrite ML application produces a single, statically linked native binary requiring no external runtime dependencies, making edge deployment trivial.
