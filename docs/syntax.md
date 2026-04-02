# Ferrite v2.0 — Syntax Reference

Ferrite v2.0 is a statically-typed, AOT-compiled ML language. All code is type-checked at compile time before any native execution.

## Basic Structure

Statements end with a semicolon `;`. Block-level constructs (`if`, `while`, `for`, `match`, `fun`) use braces and do **not** require trailing semicolons.

```ferrite
keep x: int = 10;
if x > 5 {
    keep y: int = x + 1;
}
```

## Comments

Ferrite uses `//` for single-line comments.

```ferrite
// This is a comment
keep x: int = 5; // Inline comment
```

## Variable Declarations

Variables are declared with `keep` (local immutable-intent binding) or `param` (trainable parameter).

```ferrite
keep name: string = "Ferrite";
keep version: float = 2.0;
keep is_compiled: bool = true;

param weights: Tensor<float, (784, 128)> = init();
```

> **Note:** There is no `let`, no `null`, and no dynamic typing in v2.0.

## Primitive Types

| Type     | Description              | Example         |
|:---------|:-------------------------|:----------------|
| `int`    | 64-bit signed integer    | `42`, `-10`     |
| `float`  | 64-bit floating point    | `3.14`, `-0.5`  |
| `bool`   | Boolean                  | `true`, `false`  |
| `string` | UTF-8 string             | `"Hello"`       |

## Tensor Types

Tensors carry element type and shape information at the type level:

```ferrite
param w: Tensor<float, (784, 128)> = init();
param x: Tensor<float, (B, 784)> = input();   // Symbolic dimension B
```

Shape dimensions can be constant integers or symbolic identifiers.

## Operators

### Arithmetic
`+`, `-`, `*`, `/`, `%`

### Comparison
`==`, `!=`, `<`, `>`, `<=`, `>=`

### Logical
`&&` (AND), `||` (OR), `!` (NOT)

### Unary
`-` (negation), `!` (logical not), `await` (async)

## Control Flow

### If / Elif / Else

```ferrite
if score > 90 {
    keep grade: string = "A";
} elif score > 80 {
    keep grade: string = "B";
} else {
    keep grade: string = "F";
}
```

### While Loop

```ferrite
keep i: int = 0;
while i < 10 {
    i = i + 1;
}
```

### For Loop

```ferrite
for x in items {
    process(x);
}
```

### Loop Control
- `stop;` — exits the loop (equivalent to `break`)
- `skip;` — skips to next iteration (equivalent to `continue`)

```ferrite
while true {
    if done {
        stop;
    }
    skip;
}
```

## Functions

Functions are declared with `fun` and have explicit typed parameters and return types.

```ferrite
fun add(a: int, b: int) -> int {
    return a + b;
}

fun greet(name: string) {
    // No return type means Unit
}
```

### Effect-Annotated Functions

```ferrite
infer fun predict(input: Tensor<float, (B, 784)>) -> Tensor<float, (B, 10)> {
    return forward(input);
}

train fun optimize(loss: float) -> float {
    return loss;
}
```

## Groups (Structs)

```ferrite
group Point {
    x: float;
    y: float;

    fun distance(self) -> float {
        return self.x;
    }
}

// Group literals
keep p: Point = Point { x: 1.0, y: 2.0 };
```

## Enums (Algebraic Data Types)

```ferrite
enum Color {
    Red;
    Green;
    Blue;
}

enum Option<T> {
    Some(T);
    None;
}
```

## Pattern Matching

```ferrite
match value {
    case 0 => {
        return "zero";
    }
    case 1 => {
        return "one";
    }
    default => {
        return "other";
    }
}
```

Patterns support: literals, wildcards (`_`), variable bindings, constructor patterns (`Some(x)`), and struct patterns (`Point { x, y }`).

## Constants

```ferrite
constant PI: float = 3.14159;
constant MAX: int = 1024;
```

## Generics & Trait Bounds

```ferrite
fun identity<T>(x: T) -> T {
    return x;
}

fun bounded<T: Add + Mul>(a: T, b: T) -> T {
    return a;
}

fun shaped<N: shape>(size: int) -> int
    where N > 0 {
    return size;
}
```

## ML Blocks

```ferrite
infer {
    keep output: int = predict(input);
}

train {
    keep loss: float = compute_loss();
}
```

## Imports

```ferrite
import "module_path";
import name as alias;
from "path" take function_name;
```

## Select (Structured Concurrency)

```ferrite
select {
    case result = fetch_data() => {
        process(result);
    }
    default => {
        handle_timeout();
    }
}
```
