# Language Design

## 1. Grammar Philosophy

The design of the Ferrite grammar is explicitly optimized for parser determinism, readability, and machine learning context. Ferrite heavily borrows structural idioms from Rust (brace-enclosed blocks, explicit type annotations post-fixed with a colon) and Go (clean keyword boundaries, structured concurrency), while completely rejecting Python's significant whitespace (indentation-based scoping).

**Engineering Reasoning:**
Significant whitespace is notoriously fragile in AOT-compiled toolchains, especially when auto-formatting, pasting code across network boundaries, or constructing multi-line lambda expressions. Ferrite mandates curly braces `{}` for block scopes and semicolons `;` for statement termination to guarantee that the LLVM Abstract Syntax Tree maps deterministically back to exact token boundaries regardless of formatting style.

## 2. Syntax Overview

Ferrite prioritizes explicit declaration intent.

- **Variable Declarations:** Variables are declared as immutable by default using `keep` (analogous to `let` in Rust) or parameterized (tunable) using `param`. There is no implicit dynamic binding (`x = 5` without a keyword is a syntax error unless `x` is already in scope).
- **Functions:** Declared with `fun` instead of `fn` or `function`. Explicit return types are mandatory unless the function returns void.
- **Control Flow:** `if`, `elif`, `else`, `while`, `for ... in`. Explicit loop labels are supported but implicit fallthrough in `match` statements is forbidden.
- **Machine Learning Blocks:** Built-in syntactic scopes for `train { ... }` and `infer { ... }`.
- **Modules:** `import "module"`, `from "module" take { a, b }`.

_Reference: See `docs/syntax.md` and `docs/grammar.ebnf` for comprehensive production rules._

## 3. Semantics Overview

Ferrite employs **Strict Operational Semantics** heavily biased toward deterministic numerical evaluation.

- **Eager Evaluation:** Ferrite is strictly eagerly evaluated. Thunks/Lazy evaluation must be implemented explicitly via closures.
- **Pass-by-Value (Scalars) vs Pass-by-Reference (Tensors/Groups):** To avoid massive memory copying overhead, complex types like `Tensor` and `Group` are implicitly passed by reference-counted pointer into functions, whereas scalars (`int`, `float`) are stack-copied.
- **No Implicit Coercion:** The most aggressive semantic rule in Ferrite. `1 + 1.0` is a fatal semantic error. The compiler strictly forbids hidden `int` to `float` promotion. This forces ML engineers to manually verify bit-precision boundaries (e.g., `1.0 + float(1)`).

_Reference: See `docs/semantics.md` for execution context and closure capture rules._

## 4. Type System Overview

Ferrite uses a static, nominal-leaning type system with structural subtyping exclusively reserved for Generic shape bounds.

- **Primitives:** `int` (i64), `float` (f64), `bool`, `string`.
- **Tensors:** The crown jewel of the type system. `Tensor<float, (N, M)>`. The dimensionality `(N, M)` is embedded within the type signature itself, enabling the type checker to reject invalid matrix multiplications before runtime.
- **Groups & Enums:** `group` defines product types (structs). `enum` defines algebraic sum types (ADTs) with exhaustive `match` verification.
- **Traits:** Interfaces defining required method signatures (e.g., `trait Add { fun add... }`).
- **Generics:** Type variables (`<T>`) and shape variables (`<N: shape>`). The unification engine resolves these at call sites via monomorphization.

_Reference: See `docs/type-system.md` for inference rules and unification algorithms._

## 5. Reserved Keywords

The lexer enforces exactly 34 reserved keywords. These cannot be used as variable, group, or function identifiers.

**Declaration:** `fun`, `keep`, `param`, `constant`, `group`, `enum`
**Module System:** `import`, `from`, `take`, `as`, `pub`
**Control Flow:** `if`, `elif`, `else`, `while`, `for`, `in`, `return`, `stop` (break), `skip` (continue)
**Pattern Matching:** `match`, `case`, `default`
**Machine Learning:** `infer`, `train`
**Concurrency:** `async`, `await`, `spawn`, `select`
**Types & Bounds:** `where`, `self`, `shape`
**System/FFI:** `extern`, `unsafe`

## 6. Naming Conventions

The compiler does not strictly enforce naming conventions via lints yet, but the Standard Library sets the following architectural standards:

- **Types/Groups/Enums/Traits:** `PascalCase` (e.g., `Tensor`, `NeuralNet`, `Option`).
- **Functions/Variables/Modules:** `snake_case` (e.g., `compute_loss`, `batch_size`).
- **Constants:** `SCREAMING_SNAKE_CASE` (e.g., `PI`, `MAX_EPOCHS`).

## 7. Future Evolution

The language design is intentionally conservative. Future syntax expansions will focus heavily on zero-cost abstractions:

1. **Borrow Checker / Lifetimes:** Currently, Ferrite relies on ARC (Automatic Reference Counting) for complex heap objects. A future design evolution (v3.x or v4.x) will introduce explicit lifetime syntax (e.g., `'a`) to eliminate reference counting overhead for performance-critical ML loops.
2. **Macros:** A declarative, AST-based macro system (similar to Rust's `macro_rules!`) is planned to replace boilerplate code for repeated neural network layer definitions. Text-based C-style macros `#define` have been explicitly rejected.
