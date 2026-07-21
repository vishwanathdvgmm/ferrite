# Version History

This document chronicles the architectural evolution of the Ferrite language and compiler. It tracks major paradigm shifts, technical debt introduced or resolved, and the engineering reasoning behind breaking changes.

---

## Version 2.4.1 (Current)

**Purpose:** Developer Experience (DX) and tooling stabilization.
**New Features:** None.
**Architecture Changes:**

- Refactored internal string formatting and macro expansions (`write!`) inside the AST evaluator to resolve a known desynchronization bug with Rust-Analyzer.
  **Technical Debt Removed:** Eliminated false-positive semantic errors in IDEs compiling the compiler from source.

---

## Version 2.4.0

**Purpose:** Scalability of Ferrite codebases via namespace isolation.
**New Features:**

- The Module System (`import "math"`, `from "utils" take { helper }`, `import "graphics" as gfx`).
- The `pub` visibility modifier for top-level declarations.
  **Architecture Changes:**
- Introduced a multi-pass semantic resolver. Pass 1 registers global declarations; Pass 1.5 resolves cross-file module exports; Pass 2 analyzes local expression bodies.
- Added `Type::Module` and `Value::Module` to the compiler and runtime to support namespaced field access (e.g., `math.sin`).
  **Design Changes:** All functions and variables are now strictly private to their module by default, enforcing encapsulation.
  **Trade-offs:** Increases the compilation time slightly due to the additional module resolution pass and dependency graph construction.
  **Lessons Learned:** Merging ASTs blindly (the pre-v2.4 approach) scales poorly and breaks variable shadowing rules. Namespace injection is drastically safer.

---

## Version 2.3.1

**Purpose:** Web interactivity and WASM integration.
**New Features:** Web-based Interactive Playground for Ferrite.
**Architecture Changes:** Abstracted the standard library file loading mechanism to support virtual filesystems, allowing the compiler to run entirely within a web browser via WebAssembly (WASM).
**Trade-offs:** Forced the compiler frontend to avoid synchronous OS-level file I/O operations deep within the semantic analyzer.

---

## Version 2.0.0 (The Paradigm Shift)

**Purpose:** Pivot from a dynamically typed scripting language to a statically typed, AOT-compiled ML systems language.
**New Features:**

- Strict static typing (`keep x: int = 42`).
- First-class `Tensor<Type, Shape>` primitives.
- ML execution blocks (`train`, `infer`).
- Native LLVM IR generation.
  **Architecture Changes:**
- **Complete Rewrite:** Discarded the v1.4 Bytecode VM entirely.
- Replaced the dynamic runtime type checker with a rigorous ahead-of-time semantic analysis phase.
- Introduced the `DiagnosticBag` for non-halting, multi-point error reporting.
  **Breaking Changes:** Massive syntax overhaul. Replaced `let` with `keep` and `param`. Replaced `fn` with `fun`. Removed implicit typing.
  **Trade-offs:**
- **Lost:** The extreme flexibility of dynamic scripting (e.g., heterogeneous arrays).
- **Gained:** C-level execution performance, determinism, and static tensor shape verification.
  **Technical Debt Removed:** The complex garbage collector and stack-based VM from v1.4 were entirely deleted, simplifying the runtime dramatically.
  **Lessons Learned:** A bytecode VM written in Rust cannot compete with LLVM for matrix multiplications and deep loop unrolling. To be a serious ML language, we had to adopt AOT compilation.

---

## Version 1.4.0

**Purpose:** Improve runtime performance of the original dynamic scripting language.
**New Features:** F-Strings, robust error handling (`try/catch/throw`), File I/O, stateful closures.
**Architecture Changes:** Transitioned from a slow Tree-Walking Interpreter to a Stack-Based Bytecode Virtual Machine.
**Trade-offs:** The Bytecode VM was significantly harder to debug and maintain than the tree-walker, requiring a custom opcode specification and a linear instruction pointer.
**Technical Debt Introduced:** The `Rc<RefCell<T>>` memory model used for stateful closures created reference cycles (memory leaks) that the runtime could not easily detect. This eventually led to the AOT pivot in v2.0.

---

## Version 1.0.0

**Purpose:** Proof of concept for a clean, expressive, Rust-based scripting language.
**New Features:** Basic control flow, pattern matching, anonymous lambdas.
**Architecture Changes:** A pure, single-file Tree-Walking Interpreter.
**Trade-offs:** Execution speed was abysmal due to constant AST node allocations and dynamic dispatch during evaluation.
**Lessons Learned:** While tree-walking is excellent for rapid prototyping of a language syntax, it is structurally unfit for production workloads.
