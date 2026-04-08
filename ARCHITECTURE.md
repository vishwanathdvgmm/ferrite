# Ferrite v2.1 — Compiler Architecture

This document describes the internal architecture of the Ferrite v2.1 AOT compiler.

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
│   │   └── mod.rs           # Legacy runtime (preserved, unused in v2.0)
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
│   ├── run_tests.sh         # Automated 22-test verification suite
│   ├── pass_01..10.fe       # Valid programs (must compile)
│   └── fail_01..12.fe       # Invalid programs (must be rejected)
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
           │ ferrite check             │ ferrite compile
           │ → "✅ Type-checking       │ (requires --features llvm)
           │    successful."           │
           │                           ▼
           │              ┌───────────────────────┐
           │              │  6. LLVM Codegen      │
           │              │  src/codegen/llvm.rs  │
           │              │                       │
           │              │  AST → LLVM IR        │
           │              │  inkwell bindings     │
           │              │  Output: .ll file     │
           │              └───────────────────────┘
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
