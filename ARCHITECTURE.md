# Ferrite v3.0.0 — Compiler Architecture

This document describes the internal architecture of the Ferrite v3.0.0 AOT compiler and IDE Tooling.

---

## What's New in v3.0

### 🚀 Language Server (LSP) & VS Code Extension

Ferrite now includes full IDE support powered by a native Language Server implementation.

- **Smart Compiler Discovery**: The extension auto-detects `ferrite.exe` in system `PATH` and local workspace `target/` directories, removing the need for manual configuration.
- **Diagnostics**: Real-time syntax and type errors using the semantic analyzer.
- **Auto-Formatting**: Integrated code formatter (`ferrite fmt`) accessible via VS Code's "Format on Save" feature.

### ⚡ LLVM Codegen Enhancements

The AOT compiler backend (`src/codegen/llvm.rs`) has been significantly expanded to support complex native operations that were previously fallback-only:

- **Logical Operators**: `&&` and `||` are natively compiled using LLVM control flow and Phi nodes for proper short-circuit evaluation.
- **Arithmetic**: Full support for modulo (`%`) and unary minus (`-`) directly in LLVM IR.
- **Opaque Pointers**: The compiler has been upgraded to properly utilize LLVM 15's opaque pointers (`ptr`), eliminating typed pointer deprecation warnings.

---

## What's New in v2.3

### 🌐 Interpreter Web Execution & Control Flow

The tree-walk interpreter has been significantly upgraded to support complex runtime features, enabling it to run directly in the browser on the new interactive playground.

- **Control Flow**: `stop` and `skip` are now fully evaluated in `while` loops within the interpreter.
- **Match Guards**: The interpreter now fully supports `if` guard clauses in `match` statements.
- **Closures**: Lexical environments are correctly captured at creation time for lambdas, allowing stateful callbacks.

## What's New in v2.2

### 🛠️ Trait and Impl Registries

The `TypeEnv` has been heavily expanded to act as the single source of truth for semantic rules. It now includes:

- **Trait Registry**: Stores trait definitions and required method signatures.
- **Impl Registry**: Stores implementations of traits and inherent methods for specific types.
- **Group Fields Registry**: Maps group names to their exact field types for perfect `lookup_field` resolution.
- **Enum Variants Registry**: Maps variant names to their parent enums, allowing variants to be used as global constructor functions.

### 🔄 Operator Dispatch

The semantic analyzer now converts binary expressions (e.g., `+`) into trait method dispatch (`Add.add`). It queries the `TypeEnv` to ensure the type implements the required trait before allowing the operation.

### 🧩 Match Exhaustiveness

Pass 2 of the semantic analyzer now cross-references `match` cases against the `enum_variants` registry. If cases are missing and no wildcard is present, it emits a non-fatal warning using the `DiagnosticBag`.

### 🚀 Tree-Walk Interpreter

A pure-Rust Tree-Walk interpreter has been added in `src/runtime/` to allow executing AST directly without LLVM. The `ferrite run` command now bypasses LLVM codegen entirely and instead instantiates an `Interpreter` which traverses the checked AST.

---

## What's New in v2.1

### 📚 Embedded Standard Library

Starting with v2.1, the Ferrite standard library (`math`, `strings`, `collections`, `io`) is embedded directly into the compiler binary using Rust's `include_str!` macro. This ensures that the compiler is fully self-contained and does not require external files to be present during compilation.

### 🔗 Refined Import Resolution

The `ImportResolver` has been enhanced to support virtual paths. When an `import` statement is encountered:

1. The resolver first checks if the module name matches an embedded standard library module.
2. If matched, it returns a virtual path prefixed with `<stdlib::>`.
3. If not matched, it proceeds with standard filesystem resolution relative to the current module.

### 🧩 Structural Type Unification

The `unify` engine in `types/mod.rs` has been upgraded to a recursive implementation that supports:

- **Base-Type Matching**: Allowing `GenericInst` (e.g., `List<int>`) to be initialized by `Named` group literals (`List { ... }`).
- **Call-Site Binding**: Tracking generic substitutions via a local `subst` map during function call validation.

---

## Source Tree

```
ferrite/
├── src/
│   ├── main.rs              # Compiler driver (CLI: check / compile)
│   ├── ast/
│   │   └── mod.rs           # 34+ AST node types (Program → Expr)
│   ├── codegen/
│   │   ├── mod.rs            # Codegen module root
│   │   └── llvm.rs           # LLVM IR emission via inkwell (feature-gated)
│   ├── errors/
│   │   └── mod.rs           # Span, Diagnostic, DiagnosticBag
│   ├── imports/
│   │   └── mod.rs           # DAG-based module resolution
│   ├── lexer/
│   │   ├── mod.rs           # UTF-8 character scanner
│   │   └── token.rs         # TokenKind enum (34 keywords, operators, literals)
│   ├── parser/
│   │   └── mod.rs           # Recursive descent parser (~1300 lines)
│   ├── runtime/
│   │   ├── mod.rs           # Runtime exports
│   │   ├── value.rs         # Runtime Value Enum
│   │   ├── environment.rs   # Lexical Scoping
│   │   └── interpreter.rs   # Recursive AST evaluator
│   ├── semantic/
│   │   └── mod.rs           # Scoped AST walker with type enforcement
│   ├── stdlib/
│   │   ├── mod.rs           # Embedded stdlib loader
│   │   ├── collections.fe   # Legacy v1.4 stdlib
│   │   ├── functional.fe
│   │   ├── mathutils.fe
│   │   └── strings.fe
│   └── types/
│       ├── mod.rs           # TypeEnv, Type enum, unification
│       └── tensor.rs        # TensorShape, ShapeDim, exact_match()
├── tests/
│   ├── run_tests.sh         # Automated 32-test verification suite
│   ├── pass_01..16.fe       # Valid programs (must compile)
│   └── fail_01..16.fe       # Invalid programs (must be rejected)
├── docs/
│   ├── grammar.ebnf         # Formal EBNF grammar
│   ├── syntax.md            # Language syntax reference
│   ├── semantics.md         # Compiler pipeline & operational semantics
│   ├── type-system.md       # Static type system specification
│   └── standard-library.md  # Stdlib status & migration notes
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md           # This file
├── CHANGELOG.md
├── MIGRATION.md
└── RELEASE_NOTES.md
```

## Compilation Pipeline

```
   ┌──────────────────────────────────────────────────────┐
   │                   ferrite check file.fe              │
   └──────────────────────┬───────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │    1. Lexer           │
              │    src/lexer/         │
              │                       │
              │  Source → Token[]     │
              │  34 keywords          │
              │  Span-annotated       │
              └──────────┬────────────┘
                         │
                         ▼
              ┌───────────────────────┐
              │    2. Parser          │
              │    src/parser/        │
              │                       │
              │  Token[] → AST        │
              │  Recursive descent    │
              │  Panic-mode recovery  │
              └──────────┬────────────┘
                         │
                         ▼
              ┌───────────────────────┐
              │  3. Import Resolver   │
              │  src/imports/         │
              │                       │
              │  Resolves module DAG  │
              │  Cycle detection      │
              │  Caches parsed ASTs   │
              └──────────┬────────────┘
                         │
                         ▼
              ┌───────────────────────┐
              │  4. Type Environment  │
              │  src/types/           │
              │                       │
              │  AST types → Type     │
              │  Scoped symbol table  │
              │  Unification engine   │
              └──────────┬────────────┘
                         │
                         ▼
              ┌───────────────────────┐
              │  5. Semantic Analyzer │
              │  src/semantic/        │
              │                       │
              │  Two-pass AST walk    │
              │  Pass 1: declarations │
              │  Pass 2: type check   │
              └──────────┬────────────┘
                         │
           ┌─────────────┴─────────────┐
           │ ferrite check             │ ferrite run                 │ ferrite compile
           │ → "✅ Type-checking       │ → Runs Interpreter          │ (requires --features llvm)
           │    successful."           │                             │
           │                           ▼                             ▼
           │             ┌─────────────────────────┐    ┌───────────────────────┐
           │             │ 6a. Tree-Walk Runtime   │    │ 6b. LLVM Codegen      │
           │             │ src/runtime/            │    │ src/codegen/llvm.rs   │
           │             │                         │    │                       │
           │             │ AST Evaluator           │    │ AST → LLVM IR         │
           │             │ Pure Rust               │    │ inkwell bindings      │
           │             └─────────────────────────┘    └───────────────────────┘
           │
           ▼
         Done
```

## Module Responsibilities

| Module       | File(s)                     | Responsibility                                    |
| :----------- | :-------------------------- | :------------------------------------------------ |
| **Driver**   | `main.rs`                   | CLI parsing, pipeline orchestration               |
| **Lexer**    | `lexer/mod.rs`, `token.rs`  | UTF-8 scanning, keyword recognition, tokenization |
| **Parser**   | `parser/mod.rs`             | Token stream → AST, error recovery                |
| **AST**      | `ast/mod.rs`                | All syntax tree node definitions                  |
| **Errors**   | `errors/mod.rs`             | Span, Diagnostic, DiagnosticBag, ANSI rendering   |
| **Imports**  | `imports/mod.rs`            | File resolution, DAG traversal, cycle detection   |
| **Types**    | `types/mod.rs`, `tensor.rs` | Type enum, TypeEnv, unification, tensor shapes    |
| **Semantic** | `semantic/mod.rs`           | Two-pass analysis: declaration + type checking    |
| **Codegen**  | `codegen/llvm.rs`           | LLVM IR emission (behind `llvm` feature flag)     |

## Feature Flags

| Flag   | Dependency                 | Effect                               |
| :----- | :------------------------- | :----------------------------------- |
| `llvm` | `inkwell` v0.8.0 (LLVM 15) | Enables `ferrite compile` subcommand |

When compiled **without** `--features llvm`, the compiler still fully supports `ferrite check` (parse + type-check). The LLVM codegen module is conditionally compiled out.

## Design Principles

1. **ML-First**: Language constructs (`infer`, `train`, `param`, tensor types) are first-class
2. **Strict Typing**: Zero implicit coercion, zero broadcasting, zero runtime reflection
3. **Modular**: Each compiler phase is an independent module with clean interfaces
4. **Recoverable**: Parser uses panic-mode recovery; `DiagnosticBag` collects all errors
5. **Portable**: Frontend compiles on any Rust target without requiring LLVM installed
