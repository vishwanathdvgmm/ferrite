# Semantic Analysis

## 1. Overview

The Semantic Analyzer is the core intelligence of the Ferrite compiler. While the Parser enforces structural grammar (e.g., ensuring `if` statements have braces), the Semantic Analyzer enforces meaning, validity, and type safety. It transforms a syntactically correct Abstract Syntax Tree (AST) into a strictly validated program ready for LLVM lowering or runtime interpretation.

**Core Responsibilities:**

- Scope and name resolution (ensuring variables exist before use).
- Strict type checking and unification (ensuring types match, specifically zero implicit coercion).
- Tensor shape validation (preventing `N x M` dimension mismatch errors at compile-time).
- Module visibility enforcement (`pub` vs private).

## 2. The Multi-Pass Architecture

Ferrite allows out-of-order declarations. A function defined at the bottom of a file can be called at the top of the file. To support this without complex forward-declaration syntax (like C headers), the Semantic Analyzer executes a multi-pass traversal.

### 2.1 Pass 1: Global Registration

The analyzer walks the AST and registers every `TopDecl` (`fun`, `group`, `enum`, `constant`) into the `TypeEnv`.

- Function signatures, argument types, and return types are parsed and registered.
- The _bodies_ of functions and methods are strictly ignored during this pass.

### 2.2 Pass 1.5: Import & Module Resolution

Introduced in v2.4.0, this intermediate pass resolves the cross-file Directed Acyclic Graph (DAG) built by the Import Resolver.

- It verifies that requested modules exist.
- It validates selective imports (`from "math" take { sin }`), ensuring the target symbol `sin` exists in the target module and is marked `pub`.
- It injects resolved foreign symbols into the local file's `TypeEnv`.

### 2.3 Pass 2: Type Checking & Validation

The analyzer walks the AST again, this time descending into function bodies, blocks, and expressions.

- Local variables are registered into the scoped symbol table.
- Expressions are recursively evaluated for their resulting type.
- Control flow rules are validated (e.g., `return` types match the function signature, `stop` and `skip` only exist inside loops).

## 3. Symbol Tables & Scope Resolution

### 3.1 Lexical Scoping

Ferrite implements standard lexical scoping (block scoping).
The `TypeEnv` maintains a stack of Hash Maps (`Vec<HashMap<String, Type>>`).

- When the analyzer enters a block (`{`), it pushes a new empty Hash Map onto the stack.
- Variable declarations (`keep x: int`) insert entries into the topmost map.
- When the analyzer exits the block (`}`), the topmost map is popped and discarded, instantly invalidating those variables for subsequent code.

### 3.2 Shadowing

Ferrite explicitly permits variable shadowing within nested scopes, but strictly forbids redeclaring a variable within the _same_ scope boundary.

### 3.3 Name Resolution (Lookup)

When an identifier (`x`) is encountered in an expression:

1. The analyzer iterates down the scope stack from top (deepest local scope) to bottom (global file scope).
2. If the symbol is found, its `Type` is returned.
3. If the scope stack is exhausted, the analyzer checks the imported module namespaces.
4. If still unfound, a fatal `UndefinedVariable` diagnostic is pushed to the `DiagnosticBag`.

## 4. Type Checking & Unification

Ferrite's most aggressive architectural stance is **Zero Implicit Coercion**.

### 4.1 Strict Equality

When evaluating a binary expression (`a + b`), the analyzer requests the type of `a` and the type of `b`. If `a` is `int` and `b` is `float`, the analyzer immediately pushes a `TypeMismatch` error to the `DiagnosticBag`. It explicitly does _not_ auto-promote `int` to `float`.

### 4.2 Structural Subtyping and Unification

While standard primitives rely on strict equality, complex types (like Tensors and Generics) require a **Unification Engine**.
When a function expects a generic `Tensor<float, (N, M)>` and receives a concrete `Tensor<float, (64, 128)>`:

1. The unification engine recursively matches the AST type signature against the concrete argument type.
2. It binds `N = 64` and `M = 128` in a temporary substitution map (`subst`).
3. If the function returns `Tensor<float, (N, N)>`, the engine substitutes the mapped variables, resolving the return type as `Tensor<float, (64, 64)>`.

### 4.3 Tensor Shape Validation

Tensor shapes are not just metadata; they are an intrinsic part of the type signature in the `TypeEnv`.
If a user attempts to multiply a `Tensor<float, (64, 128)>` by a `Tensor<float, (256, 128)>`, the unification engine will detect that `128 != 256` in the inner dimension and emit a compile-time dimensional mismatch error, preventing the program from ever running.

## 5. Architectural Trade-offs

### 5.1 Non-Halting Error Diagnostics

**Design:** If the analyzer finds a type error, it does not panic. It logs the error to the `DiagnosticBag` and returns a fallback `Type::Any` (or an `ErrorType`) to the current expression, allowing the analyzer to continue parsing the rest of the file.
**Trade-off:** Returning fallback types can occasionally cause cascading "ghost errors" down the line if subsequent logic relies on the failed expression. The analyzer must carefully suppress duplicate errors originating from an already poisoned AST node.

### 5.2 Pass 1.5 Module Complexity

**Design:** Resolving module namespaces requires caching parsed ASTs in the `ImportResolver` and passing their global symbols to dependent files.
**Trade-off:** This introduces significant state-sharing complexity. The compiler must ensure the global `TypeEnv` does not accidentally leak private symbols from File B into File A just because File A imported File B. The namespace injection boundary must be strictly policed.
