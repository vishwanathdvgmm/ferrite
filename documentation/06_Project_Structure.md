# Project Structure

## 1. Overview

The Ferrite compiler repository is organized as a standard Cargo workspace. The structure strictly enforces the decoupled architectural pipeline defined in `05_System_Architecture.md`. Each phase of the compilation process is encapsulated in its own directory, preventing cyclic dependencies and ensuring that the Abstract Syntax Tree (AST) remains the only shared data boundary.

## 2. Folder Structure

```text
ferrite/
├── 📁 .github
├── 📁 benchmarks
├── 📁 docs                 # Language specifications (syntax, type system, grammar)
├── 📁 documentation        # Engineering Knowledge Base (EKB)
├── 📁 target               # Compiled Rust artifacts (git-ignored)
├── 📁 src                  # Compiler source code
│   ├── 📁 ast              # Abstract Syntax Tree structures
│   ├── 📁 codegen          # Target backends (LLVM)
│   ├── 📁 errors           # Diagnostics and span tracking
│   ├── 📁 imports          # Module resolution and dependency graphs
│   ├── 📁 lexer            # Tokenizer and string interning
│   ├── 📁 parser           # Recursive descent parser
│   ├── 📁 runtime          # Tree-walking interpreter and environment
│   ├── 📁 semantic         # Type checker and declaration resolver
│   ├── 📁 stdlib           # Embedded standard library source (.fe)
│   ├── 📁 types            # Type environment, tensors, and unification
│   └── 🦀 main.rs          # CLI Driver
├── 📁 tests                # End-to-end integration and verification suites
├── 📁 website              # Project homepage, playground, and blog source
├── ⚙️ .gitignore
├── 📝 ARCHITECTURE.md
├── 📝 CHANGELOG.md
├── 📄 CNAME
├── 📝 CODE_OF_CONDUCT.md
├── ⚙️ Cargo.toml           # Dependency manifests and workspace config
├── 📄 EULA.txt
├── 📄 LICENSE
├── 📝 MIGRATION.md
├── 📝 README.md            # Project landing page
├── 📝 RELEASE_NOTES.md
├── 📝 TERMS.md
└── ⚙️ rust-toolchain.toml
```

## 3. Module Responsibilities

Each internal module under `src/` has a strictly defined responsibility and boundary.

### `src/main.rs` (The Driver)

**Responsibility:** Orchestrates the entire compilation pipeline. Parses CLI arguments (`check`, `run`, `compile`) and passes data sequentially between the lexer, parser, semantic analyzer, and backend.
**Engineering Rule:** The driver must contain zero parsing or type-checking logic. It is strictly a pipeline coordinator.

### `src/ast/` (Abstract Syntax Tree)

**Responsibility:** Defines the 34+ memory representations of parsed Ferrite code (`Expr`, `Stmt`, `TopDecl`).
**Engineering Rule:** AST nodes must be entirely pure structs and enums. They must contain no implementation logic (`impl` blocks are forbidden for behavior, allowed only for visitor pattern routing).

### `src/lexer/`

**Responsibility:** Converts raw `&str` source code into an array of `Token` structs.
**Engineering Rule:** Must never allocate strings onto the heap if possible. Uses string slicing (`&str`) to minimize memory pressure.

### `src/parser/`

**Responsibility:** Consumes tokens and outputs a `Vec<TopDecl>`.
**Engineering Rule:** Must gracefully handle missing tokens via panic-mode recovery. The parser is not permitted to halt execution on the first syntax error.

### `src/imports/`

**Responsibility:** Discovers `import` declarations in the AST, opens files from disk (or the embedded stdlib), and constructs a Directed Acyclic Graph (DAG) of module dependencies.
**Engineering Rule:** Must explicitly detect and throw errors on cyclic import dependencies (e.g., A imports B, B imports A) to prevent infinite loops in the Semantic Analyzer.

### `src/semantic/`

**Responsibility:** The brain of the compiler. Verifies type safety, enforces scoping rules, and checks tensor dimensionalities.
**Engineering Rule:** Must never mutate the AST directly. All type metadata is written to the central `TypeEnv`.

### `src/types/`

**Responsibility:** Houses the `TypeEnv` and the `Type` enum. Implements the unification engine for generics (`unify(T, U)`).
**Engineering Rule:** The unification engine must handle deeply nested structural types (e.g., `Tensor<float, (N, M)>`) safely without stack overflows.

### `src/runtime/`

**Responsibility:** Evaluates the AST locally without LLVM compilation. Manages the lexical `Environment` (stack frames and variables).
**Engineering Rule:** Must correctly replicate the strict semantic rules enforced by the semantic analyzer. Variables cannot be redeclared in the same scope.

### `src/codegen/`

**Responsibility:** Lowers the verified AST and `TypeEnv` to LLVM IR via `inkwell`.
**Engineering Rule:** Because `llvm-sys` bindings are highly platform-dependent, this module must be heavily feature-gated (`#[cfg(feature = "llvm")]`) to ensure the rest of the compiler can build without C++ toolchains.

### `src/errors/`

**Responsibility:** Houses the `DiagnosticBag`. Provides utilities for drawing ANSI-colored error arrows pointing to specific source code spans.
**Engineering Rule:** Decoupled from all other modules. Other modules only push to the bag; they do not dictate how errors are rendered to `stderr`.

## 4. Dependency Relationships

To enforce modularity, Rust's module system is used to restrict dependencies. The dependency graph flows strictly top-down:

- `lexer` depends ONLY on `errors` (for Span).
- `parser` depends on `lexer`, `ast`, and `errors`.
- `semantic` depends on `ast`, `types`, `imports`, and `errors`.
- `runtime` and `codegen` depend on `ast` and `types`.
- `ast` depends on nothing.

**Critical Constraint:** The `runtime` and `codegen` modules must never depend on each other. They are parallel backend siblings. The `semantic` module must never import backend code. This strict DAG prevents architectural spaghetti and ensures the compiler can be safely extended with new backends (e.g., WebAssembly / Cranelift) without modifying front-end code.
