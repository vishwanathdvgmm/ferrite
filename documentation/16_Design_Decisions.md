# Major Design Decisions

This document records the most consequential architectural and engineering decisions made during the evolution of the Ferrite compiler. It serves as a historical record to prevent future maintainers from endlessly debating settled topics or repeating past mistakes.

---

## Decision 1: Pivoting from Bytecode VM to LLVM (AOT Compilation)

### Problem

In Ferrite v1.4, the language ran on a custom, stack-based Bytecode Virtual Machine written in Rust. While iteration was fast, the VM could not execute dense matrix multiplications or complex neural network training loops fast enough to compete with PyTorch (C++ backends), severely limiting Ferrite's viability as an ML language.

### Alternatives Considered

1. **JIT Compilation:** Integrate a Just-In-Time compiler (like Cranelift) into the Bytecode VM to compile hot loops on the fly.
2. **C++ Transpilation:** Compile Ferrite AST to C++ source code, then shell out to `g++` to build the binary.

### Decision

Discard the Bytecode VM entirely and write an Ahead-Of-Time (AOT) backend targeting LLVM IR (Intermediate Representation) using `inkwell`.

### Reasoning

LLVM provides decades of state-of-the-art optimization algorithms (auto-vectorization, loop unrolling, register allocation). By lowering directly to LLVM IR, Ferrite immediately gains C-level execution speed for numerical workloads, achieving the project's primary performance requirements.

### Trade-offs

- **Build Complexity:** LLVM is a massive C++ dependency. Compiling the compiler now requires `llvm-sys`, which makes cross-compilation and CI pipelines significantly more fragile.
- **Lost Dynamism:** We had to abandon dynamic typing entirely in v2.0 because LLVM requires strict static memory layouts to optimize effectively.

### Long-term Consequences

Ferrite became a true systems programming language capable of bare-metal performance, but the barrier to entry for contributing to the code generator (`src/codegen/llvm.rs`) increased dramatically.

### Would this still be the recommended decision today?

**Yes.** To be a viable alternative to C++/CUDA for machine learning, bare-metal native code generation is non-negotiable.

---

## Decision 2: Zero Implicit Type Coercion

### Problem

In most languages (C, Python, Java), adding an integer to a float (`1 + 3.14`) silently coerces the integer into a float before performing the addition. In machine learning, silent precision shifts (e.g., implicitly casting an `f64` tensor to an `f32` tensor) can destroy gradient calculations, causing models to diverge mysteriously.

### Alternatives Considered

1. **Standard Promotion:** Follow C rules and auto-promote smaller types to larger types.
2. **Warning on Coercion:** Allow the coercion but emit a compiler warning.

### Decision

Strictly forbid all implicit type coercion. `1 + 3.14` is a fatal compile-time semantic error.

### Reasoning

Explicit is better than implicit. By forcing the developer to write `float(1) + 3.14`, the compiler guarantees that every precision shift in a mathematical formula is intentional and audited by the human engineer.

### Trade-offs

The language feels slightly more verbose and "pedantic" than Python. Developers coming from Python may find the initial strictness frustrating when writing simple math scripts.

### Long-term Consequences

Entire categories of numerical instability bugs in complex ML algorithms were eliminated before execution.

### Would this still be the recommended decision today?

**Yes.** The friction introduced during writing is vastly outweighed by the hours saved debugging silent precision loss during training.

---

## Decision 3: Side-Loading AST Metadata (Rejecting `TypedAST`)

### Problem

After the Parser builds the raw `AST`, the Semantic Analyzer must resolve variable types, enforce scopes, and map function calls to specific signatures. Where should this resolved metadata be stored?

### Alternatives Considered

1. **Mutable AST:** Wrap every node in `Rc<RefCell<Node>>` so the semantic analyzer can mutate the tree and attach types directly to the nodes.
2. **TypedAST Mapping:** Create a parallel set of Structs (e.g., `TypedExpr`, `TypedStmt`) and have the semantic analyzer transform the raw AST into this new tree.

### Decision

Keep the raw `AST` strictly immutable. Store all resolved types and symbol metadata in an external Hash Map called the `TypeEnv`, keyed by unique node locations or IDs.

### Reasoning

Mutating a massive tree in Rust requires locking mechanisms (`RefCell`) which incur runtime overhead and panic risks. Building a completely new `TypedAST` requires duplicating thousands of heap allocations (`Box`), destroying CPU cache locality and slowing down compilation. Side-loading into a central `TypeEnv` preserves memory and avoids structural duplication.

### Trade-offs

The backend (Interpreter/LLVM) must constantly query the `TypeEnv` during code generation, performing a hash lookup for every node instead of simply reading a pointer field.

### Long-term Consequences

Compilation speed in the front-end remains blazing fast, but the code generator logic is slightly more complex due to the decoupled state.

### Would this still be the recommended decision today?

**Mixed.** As the AST grows, hash lookups become expensive. A modern revision (planned for v3.x) should inject a sequential `NodeId` into every AST node during parsing, allowing the `TypeEnv` to be a flat array (`Vec<Type>`) indexed by `NodeId`, replacing $O(1)$ Hash Map lookups with $O(1)$ direct array access.

---

## Decision 4: `train` and `infer` as Compiler Primitives

### Problem

Most ML frameworks (PyTorch) manage execution context via library-level context managers (e.g., `with torch.no_grad():`). This relies on dynamic runtime state, which hinders AOT optimization because the compiler cannot statically guarantee whether a function is running in inference or training mode.

### Alternatives Considered

1. **Library Functions:** Use `ferrite.set_grad_enabled(false)`.
2. **Function Attributes:** Require developers to annotate functions with `@[infer]`.

### Decision

Introduce `train { ... }` and `infer { ... }` as built-in syntax keywords (block expressions) in the Ferrite grammar.

### Reasoning

By pushing the ML context directly into the AST, the compiler's Semantic Analyzer knows _statically_ which blocks require gradient computation graphs and which do not. During LLVM lowering, the compiler can physically strip out gradient-tracking memory allocations for code inside an `infer` block, yielding a significantly smaller and faster binary.

### Trade-offs

Language complexity increases. The language is now explicitly domain-specific (Machine Learning) rather than purely general-purpose.

### Long-term Consequences

Ferrite has a massive architectural advantage over library-based ML frameworks in deployment scenarios (Edge AI) where memory and binary size are strictly constrained.

### Would this still be the recommended decision today?

**Yes.** This is the defining architectural feature of Ferrite.
