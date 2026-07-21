# Error Handling

## 1. Overview

Compiler error handling is frequently the most neglected aspect of language design, yet it dictates the entire Developer Experience (DX). In Ferrite, error handling is treated as a core architectural tier, structurally decoupled from the parsing and semantic logic that detects the errors.

The architecture strictly distinguishes between **Compile Errors** (static violations detectable before execution) and **Runtime Errors** (logical or state violations occurring during execution).

## 2. Compile Errors & The DiagnosticBag

Ferrite enforces non-halting, multi-point error reporting during compilation. When the compiler encounters an error (like a missing semicolon or a type mismatch), it explicitly avoids aborting the Rust process via `panic!()` or early `return Err()`.

### 2.1 The `DiagnosticBag`

The central nervous system of compiler error handling is the `DiagnosticBag`. It is a mutable struct passed by reference (`&mut DiagnosticBag`) through every phase of the front-end (Lexer, Parser, Import Resolver, Semantic Analyzer).

**Core Responsibilities:**

- Accumulating errors across isolated compiler modules.
- Preventing duplicate errors from cascading (e.g., if a variable fails to resolve, suppressing the subsequent "Type Mismatch" errors involving that variable).
- Mapping physical source code byte offsets (`Span`) back to readable lines and columns.

### 2.2 Error Types

The `DiagnosticBag` tracks a unified `Diagnostic` enum, categorizing failures into:

1. **Lexical Errors:** Unrecognized tokens or unclosed string literals.
2. **Syntactical Errors:** Unexpected tokens, missing braces, invalid expressions.
3. **Semantic Errors:** Type mismatches, undefined variables, visibility (`pub`) violations, dimensionality clashes, unsupported trait implementations.

## 3. Error Reporting

Once the Semantic Analyzer completes its final pass, the CLI Driver (`src/main.rs`) inspects the `DiagnosticBag`. If the bag is not empty, compilation fails, and the errors are flushed to the terminal (`stderr`).

### 3.1 ANSI Output Rendering

To maximize readability, Ferrite mimics Rust's highly acclaimed `rustc` error rendering.
When a diagnostic is flushed, the renderer:

1. Reads the `Span` attached to the error.
2. Extracts the exact line of offending code from the original source string.
3. Prints the context, applying ANSI color codes to highlight the specific token, with a descriptive message explaining _why_ it failed.

**Example Rendering:**

```text
error: Type mismatch: expected 'int', found 'float'. Implicit coercion is forbidden.
  --> src/math.fe:14:22
   |
14 |     keep x: int = 3.14;
   |                   ^^^^
```

## 4. Recovery Mechanisms

Reporting multiple errors in a single compiler pass requires aggressive error recovery.

### 4.1 Parser Panic-Mode Recovery

As outlined in `09_Parser.md`, when the Parser encounters a syntax error, it pushes a diagnostic and enters "panic mode". It discards incoming tokens until it identifies a synchronization boundary (like a `;` or a `}`), effectively isolating the syntactical corruption to a single statement and allowing the rest of the file to parse cleanly.

### 4.2 Semantic Fallback Types

When the Semantic Analyzer catches a type mismatch (e.g., `1 + "hello"`), the expression is structurally invalid. To prevent the rest of the analysis from crashing due to an unresolved type, the analyzer pushes the error to the `DiagnosticBag` and artificially assigns the expression a fallback `Type::Any` (or a dedicated `Type::Error`).
Subsequent type checks encountering `Type::Any` silently succeed without emitting further diagnostics, effectively arresting cascading ghost errors.

## 5. Runtime Errors

If the AST passes semantic validation, mathematical and structural type safety is guaranteed. However, runtime panics can still occur due to logical state violations.

### 5.1 Checked Operations

During Interpreter execution or LLVM execution, the runtime explicitly traps:

- **Division by Zero:** Evaluated natively and trapped to prevent hardware faults.
- **Index Out-of-Bounds:** Array and Tensor accesses are strictly bounds-checked against the runtime size of the allocation.
- **Out of Memory (OOM):** Handled by the host OS or Rust's global allocator.

### 5.2 Aborting vs Catching

Currently, Ferrite strictly aborts the process on runtime faults like Out-Of-Bounds array access, printing a stack trace.

**Architectural Decision:**
In earlier versions (v1.4), Ferrite supported `try/catch/throw` exception handling. This was deliberately removed in the v2.0 AOT rewrite. Exceptions introduce massive overhead in LLVM (via unwinding tables or setjmp/longjmp) and obscure control flow. Future iterations of Ferrite will adopt a strict algebraic `Result<T, E>` error model (akin to Rust and Zig) for recoverable logic, reserving process-abort strictly for unrecoverable hardware faults.
