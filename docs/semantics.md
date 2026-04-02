# Ferrite v2.0 — Semantics

This document describes the operational semantics and compiler pipeline of Ferrite v2.0.

## Compiler Pipeline

Ferrite v2.0 is an ahead-of-time (AOT) compiled language. Source code passes through the following stages:

```
Source (.fe)
    │
    ▼
┌─────────┐
│  Lexer  │  src/lexer/     Tokenizes UTF-8 source into span-annotated tokens
└────┬────┘
     ▼
┌─────────┐
│ Parser  │  src/parser/    Recursive descent parser with panic-mode recovery
└────┬────┘
     ▼
┌──────────┐
│   AST    │  src/ast/      Strongly typed syntax tree (34+ node types)
└────┬─────┘
     ▼
┌───────────────┐
│ ImportResolver│  src/imports/  DAG-based module resolution with cycle detection
└────┬──────────┘
     ▼
┌───────────┐
│  TypeEnv  │  src/types/   Structural type unification, tensor shape validation
└────┬──────┘
     ▼
┌──────────────────┐
│ SemanticAnalyzer │  src/semantic/  Scoped type checking, invariant enforcement
└────┬─────────────┘
     ▼
┌──────────────┐
│ LLVM Codegen │  src/codegen/  Native code emission via inkwell (feature-gated)
└──────────────┘
```

## Static Typing

Ferrite v2.0 is **statically and strictly typed**. All type errors are caught at compile time.

### No Implicit Coercion

```ferrite
keep x: int = 3.14;    // ❌ Error: Type mismatch: expected 'int', found 'float'
keep y: float = 42;    // ❌ Error: no implicit int → float promotion
```

### No Runtime Reflection

Type introspection functions like `typeof()` or dynamic casting do not exist. The `SemanticAnalyzer` actively rejects any attempt at runtime type inspection.

## Scoping Rules

Ferrite uses **lexical scoping** with a stack of hash maps in the `TypeEnv`:

- `enter_scope()` pushes a new frame
- `exit_scope()` pops it
- Variable lookup walks the scope stack from innermost to outermost
- Redeclaring a variable in the same scope is a compile error
- Shadowing across scopes is allowed

```ferrite
fun example() {
    keep x: int = 1;          // Scope 1
    if true {
        keep x: int = 2;      // Scope 2 — shadows, allowed
        keep y: int = x + 1;  // y = 3
    }
    // y is not accessible here
}
```

## Variable Declarations

### `keep` — Local Immutable-Intent Binding

```ferrite
keep x: int = 42;
```

Declares a typed local variable. The name, type, and initializer are all mandatory. Reassignment is allowed for now (mutability enforcement planned for future versions).

### `param` — Trainable Parameter

```ferrite
param w: Tensor<float, (784, 128)> = init();
```

Semantically identical to `keep` at the type level, but signals to the ML runtime that this value participates in gradient computation during `train` blocks.

## Function Semantics

- Functions are declared with `fun` and have explicitly typed parameters
- Return type is mandatory if the function returns a value
- Functions without `-> type` return `Unit`
- `return` outside a function body is a compile error
- All top-level functions are forward-declared in Pass 1, allowing mutual recursion

## Effect System

Functions can be annotated with effects that constrain their execution context:

| Effect  | Meaning                                    |
|:--------|:-------------------------------------------|
| `infer` | Function runs in inference-only mode       |
| `train` | Function participates in training/gradient |
| `async` | Function is asynchronous                   |

## Tensor Shape Semantics

Tensor shapes use **exact structural matching**:

```ferrite
// These are DIFFERENT types:
Tensor<float, (784, 128)>
Tensor<float, (128, 784)>

// Symbolic dimensions match by name:
Tensor<float, (B, 784)> == Tensor<float, (B, 784)>  // ✅
Tensor<float, (B, 784)> == Tensor<float, (N, 784)>  // ❌ B ≠ N
```

**No implicit broadcasting. No implicit reshaping.** Shape mismatches are compile errors.

## Pattern Matching

The `match` statement evaluates a subject expression and checks each `case` arm's pattern:

- **Literal patterns**: matched by value equality
- **Wildcard `_`**: matches anything, binds nothing
- **Binding**: matches anything, binds the value to a name in the case scope
- **Constructor**: e.g., `Some(x)` — matches ADT variants
- **Struct**: e.g., `Point { x, y }` — matches group fields

## Error Recovery

The parser uses **panic-mode recovery**:

1. On encountering a syntax error, it enters panic mode
2. Suppresses further errors until a synchronization point is found
3. Synchronization tokens: `fun`, `keep`, `param`, `constant`, `group`, `enum`, `import`, `if`, `while`, `for`, `match`, `return`, `stop`, `skip`, or `;`
4. This prevents cascading phantom errors from a single typo

## Diagnostics

All errors are collected in a `DiagnosticBag` and emitted after each compilation phase:

- ANSI-colored output with `error:`, `warning:`, `note:` prefixes
- Source line display with caret (`^`) pointing to the exact token
- Error count summary: `"compilation failed with N errors and M warnings"`
