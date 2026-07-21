# Architect Journal

This journal serves as the chronological engineering diary for the Ferrite compiler. It records the specific context, constraints, and debates occurring at the exact moment major architectural decisions were finalized.

---

## Entry: The Genesis of Ferrite (v1.0.0)

**Date:** March 15, 2026
**Context:** The project began as an experimental, dynamically typed scripting language written in Rust to test AST traversal speeds.
**Problem:** Need a rapid prototyping environment to evaluate whether a syntax blending Rust-like blocks and Python-like simplicity is viable.
**Alternatives:**

1. Build a bytecode compiler immediately.
2. Build a simple Tree-Walking Interpreter.
   **Final Decision:** Built a pure Tree-Walking Interpreter.
   **Engineering Reasoning:** A tree-walker is trivial to implement using Rust's pattern matching (`match ast_node`). It allowed the team to rapidly iterate on the grammar and parsing logic without worrying about opcode specifications or memory offsets.
   **Trade-offs:** Execution speed was abysmal. Every loop iteration required dynamic dispatch and hash map lookups.
   **Future Risks:** The runtime was inherently unscalable for numerical computing.
   **Lessons Learned:** Tree-walking is excellent for syntax validation and prototyping, but it is structurally incapable of reaching C-level performance.

---

## Entry: The Bytecode Pivot (v1.4.0)

**Date:** May 10, 2026
**Context:** Users attempted to run nested `while` loops for data parsing, and the Tree-Walker was timing out.
**Problem:** The language needed a massive performance injection to remain viable.
**Alternatives:**

1. Lower AST to LLVM IR.
2. Build a custom Stack-Based Bytecode VM.
   **Final Decision:** Built a Stack-Based Bytecode Virtual Machine.
   **Engineering Reasoning:** At the time, the language was dynamically typed. LLVM is notoriously hostile to dynamic typing. A custom VM allowed us to keep the language dynamic while flattening the AST execution into a linear array of fast `u8` opcodes.
   **Trade-offs:** The compiler logic split into two massive components: a Bytecode Emitter and a Virtual Machine loop.
   **Future Risks:** We implemented stateful closures by capturing lexical environments using `Rc<RefCell<Environment>>`. This created a massive risk of memory leaks via reference cycles, as we deliberately avoided writing a Garbage Collector to maintain predictable latency.
   **Lessons Learned:** Managing a custom VM is a maintenance nightmare. Fixing off-by-one errors in instruction pointers drained engineering resources that should have been spent on language features.

---

## Entry: The ML AOT Rewrite (v2.0.0)

**Date:** June 05, 2026
**Context:** The goal of the language shifted heavily toward Machine Learning and Systems Programming. We needed C-level performance.
**Problem:** The Bytecode VM was fast, but not fast enough to compete with PyTorch's C++ backends for matrix multiplications. Furthermore, dynamic typing was causing late-stage dimension mismatch crashes in ML models.
**Alternatives:**

1. Add JIT (Just-In-Time) compilation to the VM.
2. Rewrite the language to be statically typed and compile to LLVM.
   **Final Decision:** Discard the VM. Rewrite the language to enforce strict static typing and lower directly to LLVM IR via `inkwell`.
   **Engineering Reasoning:** We cannot beat LLVM's auto-vectorization and optimization passes. By forcing the developer to provide static types, the compiler can guarantee tensor dimensions at compile time, eliminating a massive category of ML bugs.
   **Trade-offs:** We had to abandon dynamic typing. The language became significantly more rigid. The build pipeline became fragile on Windows due to LLVM C++ dependencies.
   **Future Risks:** We split the ecosystem. The LLVM feature had to be feature-gated (`#[cfg(feature = "llvm")]`), meaning we had to rebuild the old Tree-Walker Interpreter just to allow users without C++ toolchains to run Ferrite locally.
   **Lessons Learned:** Radical pivots are sometimes necessary. The AOT rewrite saved the project from becoming just another slow dynamic scripting language.

---

## Entry: The Playground & Virtual File Systems (v2.3.1)

**Date:** July 10, 2026
**Context:** Ferrite was gaining traction, but requiring users to download a Windows executable hindered adoption.
**Problem:** We needed to compile the Ferrite compiler itself to WebAssembly (WASM) to run in the browser. However, the Semantic Analyzer relied on synchronous OS file reads (`std::fs::read_to_string`) to load the standard library.
**Alternatives:**

1. Use asynchronous JavaScript interop for file reads.
2. Embed the standard library directly into the compiler binary.
   **Final Decision:** Embed the standard library using Rust's `include_str!` macro.
   **Engineering Reasoning:** By embedding the `.fe` files into the Rust binary at compile time, the Ferrite Semantic Analyzer never needs to touch the OS filesystem to resolve standard imports. The compiler remains fully synchronous and portable to WASM.
   **Trade-offs:** The compiler binary size increased by several kilobytes.
   **Future Risks:** As the standard library grows, compiling the compiler will take longer, and the binary size will bloat.
   **Lessons Learned:** Dependency injection (abstracting the file system) is critical for portability.

---

## Entry: Namespace Encapsulation (v2.4.0)

**Date:** July 20, 2026
**Context:** Users were building larger projects spanning multiple files. The old import system blindly merged ASTs, causing variable shadowing conflicts and symbol pollution.
**Problem:** We needed a strict module system to isolate scopes.
**Alternatives:**

1. Enforce a one-class-per-file rule (Java style).
2. Implement a scoped namespace system with explicit `pub` exports (Rust/Go style).
   **Final Decision:** Implement the `pub` keyword and require explicit selective imports (`from "math" take { sin }`).
   **Engineering Reasoning:** To scale to 100,000-line codebases, developers must know exactly where a symbol originated. The Semantic Analyzer was upgraded to a multi-pass system (Pass 1, Pass 1.5, Pass 2) to build a dependency DAG and cross-reference exports.
   **Trade-offs:** Pass 1.5 introduces slight compilation overhead, and developers must type `pub` repetitively.
   **Future Risks:** The DAG logic must carefully track cyclic dependencies, or the compiler will enter an infinite recursion loop during Import Resolution.
   **Lessons Learned:** AST merging is dangerous. Explicit module isolation is the only way to scale safely.
