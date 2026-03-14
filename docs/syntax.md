# Ferrite Syntax

Ferrite uses a clean, C-style syntax designed for expressiveness and readability while remaining extremely easy to parse.

## Basic Structure
Statements must end with a semicolon `;`, except for block-level expressions like `if`, `while`, `for`, `match`, and `fn` definitions, which optionally do not require semicolons.

```ferrite
let a = 10;
if a > 5 {
    print("Large");
}
```

## Comments
Ferrite uses Python-style `#` for comments. There are no block comments.

```ferrite
# This is a comment
let x = 5; # In-line comment
```

## Variables
Variables are declared using the `let` keyword and are dynamically typed.

```ferrite
let name = "Ferrite";
let version = 1.4;
let is_fast = true;
```

## Data Types (Literals)
Ferrite supports the following primitive literals:

- **Integers:** `42`, `-10`
- **Floats:** `3.14`, `-0.5`
- **Booleans:** `true`, `false`
- **Strings:** `"Hello"` (Double quotes only)
- **F-Strings:** `f"Value: {x + 1}"` (Python style)
- **Null:** `null`

### Composite Types
- **Lists:** `[1, 2, "three", true]`
- **Maps (Dicts):** `{"key": "value", "count": 42}`

## Operators

### Arithmetic
`+`, `-`, `*`, `//` (integer division), `/` (float division), `%` (modulo), `**` (exponentiation)

### Comparison
`==`, `!=`, `<`, `>`, `<=`, `>=`

### Logical
`&&` (AND), `||` (OR), `!` (NOT)

### Null Coalescing
`??` (Returns right side if left side is `null`)
```ferrite
let port = config["port"] ?? 8080;
```

## Control Flow

### If / Else If / Else
`if` statements are expressions and return the value of their evaluated block.

```ferrite
let grade = if score >= 90 { 
    "A" 
} else if score >= 80 { 
    "B" 
} else { 
    "F" 
};
```

### Match
A powerful pattern-matching construct.

```ferrite
match score {
    100 => print("Perfect"),
    90..99 => print("A"),
    _ => print("Other")
}
```

### Loops
**While Loop:**
```ferrite
while condition { ... }
```

**For Loop (Iterators):**
Can iterate over lists, strings, or dynamically. Destructuring `[a, b]` is natively supported.
```ferrite
for x in [1, 2, 3] { print(x); }
for [i, v] in enumerate(["a", "b"]) { ... }
```

## Functions
Functions are declared with `fn`. They support variadic arguments (`...args`).

```ferrite
fn greet(name, ...titles) {
    return "Hello " + name;
}
```

Lambdas are anonymous functions:
```ferrite
let add = fn(a, b) { return a + b; };
```

## Error Handling
Exceptions are caught beautifully without panicking the host process.

```ferrite
try {
    throw "Error occurred";
} catch err {
    print("Caught: " + err);
}
```

## Modules
Ferrite handles modules simply via `import`. It searches local files first or instantly loads statically compiled internal standard library scripts.
```ferrite
import "mathutils";
```
