# Ferrite v2.0 — Type System

Ferrite v2.0 is **statically and strictly typed**. All types are resolved at compile time by the `TypeEnv` (type environment) and enforced by the `SemanticAnalyzer`. There is no runtime type information, no implicit coercion, and no dynamic dispatch.

## Primitive Types

| Type     | Rust Backing | Description              |
|:---------|:-------------|:-------------------------|
| `int`    | `i64`        | 64-bit signed integer    |
| `float`  | `f64`        | 64-bit floating point    |
| `bool`   | `bool`       | `true` or `false`        |
| `string` | `String`     | UTF-8 heap-allocated     |

## Tensor Type

```
Tensor<element_type, (dim1, dim2, ...)>
```

Tensors are parameterized by their element type and shape tuple:

```ferrite
Tensor<float, (784, 128)>     // Constant shape
Tensor<float, (B, C, H, W)>  // Symbolic dimensions
Tensor<int, (64)>             // 1D integer tensor
```

### Element Type Restriction

Only `int` and `float` are valid tensor element types. Using any other type (e.g., `bool`, `string`, a named type) produces a compile error:

```
error: Tensors can only contain 'int' or 'float', not 'bool'
```

### Shape Matching

Shape matching is **exact and structural**:
- Constant dimensions match by value: `784 == 784`
- Symbolic dimensions match by name: `B == B`, but `B ≠ N`
- No implicit broadcasting or reshaping — mismatches are errors

## Composite Types

### Named Types

User-defined types created via `group` or `enum` declarations:

```ferrite
group Point { x: float; y: float; }   // Type: Point
enum Color { Red; Green; Blue; }       // Type: Color
```

### Generic Types

Parameterized types with type arguments:

```ferrite
enum Option<T> { Some(T); None; }     // Type: Option
group Container<T> { value: T; }       // Type: Container
```

## Special Types

| Type    | Description                                            |
|:--------|:-------------------------------------------------------|
| `Unit`  | Return type of functions with no `-> type` annotation  |
| `Never` | Type of divergent expressions (`stop`, `skip`)         |
| `Error` | Internal sentinel for failed type checks; suppresses cascading errors |

## Type Unification

The `unify(expected, actual)` function enforces structural equality:

```
unify(int, int)       → ✅ OK
unify(int, float)     → ❌ "Type mismatch: expected 'int', found 'float'. Implicit coercion is forbidden."
unify(Error, _)       → ✅ Suppressed (prevents cascade)
unify(Never, _)       → ✅ Never unifies with everything
```

### Tensor Unification

Tensor unification checks both element type and shape:

```
unify(Tensor<float, (784, 128)>, Tensor<float, (784, 128)>)  → ✅
unify(Tensor<float, (784, 128)>, Tensor<float, (128, 784)>)  → ❌ Shape mismatch
unify(Tensor<float, (B, 784)>,   Tensor<int,   (B, 784)>)    → ❌ Element mismatch
```

## Operator Type Rules

### Arithmetic (`+`, `-`, `*`, `/`, `%`)

Both operands must be the **same** numeric type:
- `int OP int → int`
- `float OP float → float`
- `int OP float → ❌ Error` (no promotion)

### Comparison (`<`, `>`, `<=`, `>=`, `==`, `!=`)

Both operands must be the same type. Result is always `bool`.

### Logical (`&&`, `||`)

Both operands must be `bool`. Result is `bool`.

### Unary (`-`, `!`)

- `-expr` requires `int` or `float`
- `!expr` requires `bool`

## Type Resolution

The `TypeEnv.resolve_ast_type()` function converts AST type nodes into resolved `Type` values:

| AST Type                          | Resolved Type                    |
|:----------------------------------|:---------------------------------|
| `Primitive(Int)`                  | `Type::Int`                      |
| `Named("Point")`                 | `Type::Named("Point")`          |
| `Tensor { elem, shape }`         | `Type::Tensor(elem, TensorShape)` |
| `Generic { name: "Option", .. }` | `Type::Named("Option")`         |

## Scope-Based Symbol Table

The `TypeEnv` maintains a stack of scopes (`Vec<HashMap<String, Type>>`):

- **Declaring** a variable adds it to the current (top) scope
- **Looking up** a variable searches from innermost scope outward
- **Redeclaration** in the same scope is an error
- **Shadowing** across scopes is permitted
