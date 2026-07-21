# Project Overview

## 1. Executive Summary

Ferrite is a statically-typed, ahead-of-time (AOT) compiled systems programming language designed explicitly for modern machine learning and high-performance computing. Built entirely in Rust, the Ferrite compiler lowers to LLVM (or runs via an internal AST-walking interpreter for rapid prototyping), providing near C-level performance while maintaining a highly ergonomic, ML-centric syntax.

At its core, Ferrite treats multi-dimensional arrays (Tensors), automatic differentiation, and structured concurrency as first-class language primitives rather than library add-ons, fundamentally reshaping how neural architectures and parallel workloads are expressed.

## 2. Vision

The long-term vision of Ferrite is to bridge the "two-language problem" in machine learning. Historically, data scientists and ML engineers prototype in high-level, dynamic languages (like Python) but rely on opaque, complex C/C++ backends for execution.

Ferrite envisions a unified ecosystem where the language used to design an AI model is the exact same language that executes it optimally on bare metal or specialized hardware (GPUs/TPUs). The compiler should natively understand the shapes and constraints of tensors at compile-time to prevent the vast majority of runtime tensor dimension mismatches.

## 3. Philosophy

Ferrite is governed by strict engineering principles:

- **Zero Implicit Coercion:** Ferrite strictly prohibits implicit type conversions (e.g., silently coercing an `int` to a `float`). If data must change shape or precision, the programmer must explicitly command it. This eliminates a massive category of silent precision-loss bugs in ML math.
- **Compile-Time Determinism:** If a neural network architecture is structurally flawed (e.g., matrix multiplication dimension mismatch), the compiler must fail before the program ever runs.
- **Native ML Semantics:** Concepts like `train`, `infer`, and `Tensor` are keywords and intrinsic compiler types, allowing the backend to aggressively optimize memory layouts and dispatch directly to hardware accelerators without FFI overhead.
- **Fearless Concurrency:** Borrowing heavily from Go and Rust, Ferrite implements structured concurrency (`async`, `await`, `spawn`, `select`) directly into the runtime, making high-throughput data pipelining trivial and safe.

## 4. Goals

- **Performance:** Achieve execution speeds within 5% of optimized C++.
- **Safety:** Prevent runtime tensor shape errors via advanced compile-time shape inference and dependent-like typing for dimensions.
- **Ergonomics:** Provide a syntax that feels as fluid as Python/Swift while retaining the rigorous type safety of Rust.
- **Self-Hosting (Eventual):** The ultimate goal is for the Ferrite compiler to be written in Ferrite, proving the language's viability for complex systems engineering.

## 5. Non-Goals

- **Dynamic Typing:** Ferrite will never support dynamic typing or duck-typing. Everything must be statically verifiable.
- **Garbage Collection (Traditional):** Ferrite avoids stop-the-world JVM-style garbage collection. Memory is managed deterministically via a hybrid ARC (Automatic Reference Counting) and ownership model.
- **Web/Frontend Development:** While WebAssembly compilation is a technical possibility via LLVM, Ferrite is not designed for DOM manipulation or UI rendering. It is strictly a backend, ML, and systems tool.

## 6. Target Users

- **Machine Learning Engineers:** Professionals building custom neural architectures who need fine-grained control over memory and execution speed without dropping into C++/CUDA.
- **Data Infrastructure Engineers:** Developers building high-throughput data pipelines, feature stores, and distributed training systems.
- **Systems Programmers:** Engineers who appreciate Rust's safety but prefer a slightly higher-level syntax focused on numerical computing and concurrency.

## 7. Use Cases

1. **Custom AI Model Training/Inference:** Building novel transformer architectures or CNNs where standard Python libraries (PyTorch/TensorFlow) introduce too much overhead or lack flexibility for custom hardware deployment.
2. **High-Frequency Trading (HFT):** Low-latency systems requiring predictable, jitter-free execution times with advanced math capabilities.
3. **Embedded ML (Edge AI):** Compiling tiny, standalone static binaries containing fully trained inference models to run on resource-constrained IoT devices.
4. **Data ETL Pipelines:** Using Ferrite's `spawn` and `select` concurrency primitives to safely ingest, transform, and stream massive datasets in parallel.

## 8. Unique Features

- **First-Class Tensors:** `Tensor<float, (64, 128)>` is a primitive type known to the compiler, enabling deep loop unrolling and vectorization optimizations during LLVM lowering.
- **Built-in `train` and `infer` Blocks:** Contextual blocks that automatically shift the compiler's optimization strategies. Code inside `train` retains computation graphs for autodiff, while `infer` aggressively strips metadata to minimize memory footprint.
- **Shape Constraints in `where` Clauses:** Functions can constrain generic shapes: `fun matmul<N, M, P>(a: Tensor<float, (N, M)>, b: Tensor<float, (M, P)>) -> Tensor<float, (N, P)>`.
- **Module System:** A newly introduced (v2.4.0) namespace and visibility system (`pub`, `import`, `from ... take`) that enables scalable codebase architecture without symbol pollution.
