# Ferrite Migration Guide

This document covers major changes and migration steps between Ferrite versions.

## Migrating from Ferrite v2.4.1 to v3.0.0

Ferrite v3.0.0 introduces official IDE tooling (VS Code Extension, Language Server, and Formatter) and enhances the AOT LLVM compiler backend. There are no breaking syntax changes in this release.

### 1. IDE Tooling

You no longer need to rely purely on CLI errors. Install the official "Ferrite Programming Language" extension (v1.1.0+) in VS Code or VSCodium. The extension automatically discovers your compiler in the system `PATH` or the local workspace, providing real-time diagnostics and formatting on save.

### 2. Native LLVM Operators

Operations that previously required workarounds or crashed the AOT compiler are now natively supported in LLVM IR when running `ferrite compile`. You no longer need to use hacks for:

- **Logical AND / OR**: `&&` and `||` now compile cleanly.
- **Modulo**: `%` is natively supported for integer remainders.
- **Unary Minus**: `-x` is natively supported (previously required `0 - x`).

---

## Migrating from Ferrite v2.2.1 to v2.3.1

Ferrite v2.3.1 focuses on bringing the interpreter's capabilities to parity with the AOT compiler, specifically regarding closures and control flow. There are no breaking syntax changes in this release, but behavior inside `ferrite run` is vastly improved.

### 1. Closures and State

Lambdas now correctly capture their environment. In v2.2.1, using an outer variable inside a lambda could result in undefined behavior during interpreter execution. This is now fully supported.

### 2. Match Guards

You can now use `if` guards on match cases when using `ferrite run` or the web playground.

```ferrite
match num {
    case x if x < 0 => { println("Negative"); }
    default => { println("Positive"); }
}
```

### 3. Web Playground

You can now experiment with Ferrite v2.3.1 directly in your browser at [ferrite-lang.org](https://ferrite-lang.org/) without installing the compiler locally.

---

## Migrating from Ferrite v2.1 to v2.2.1

Ferrite v2.2.1 introduces full trait support, strict trait bound enforcement, and a new tree-walk interpreter.

### 1. Tree-Walk Interpreter

You can now use `ferrite run script.fe` to execute your programs without needing the LLVM C++ toolchain installed on your machine. This is a massive shift for Windows users who may not have Visual Studio C++ libraries installed.

### 2. Operator Overloading

Operators (`+`, `-`, `*`, `/`, `%`) on user-defined types now strictly require the implementation of corresponding traits (`Add`, `Sub`, `Mul`, `Div`, `Mod`).

**v2.1 Approach:** Implicit coercion or errors depending on context.
**v2.2 Approach:** You must implement the trait.

```ferrite
trait Add { fun add(self, other: Self) -> Self; }

impl Add for Point {
    fun add(self, other: Self) -> Self { ... }
}
```

### 2. Match Exhaustiveness

`match` statements on `enum` types will now generate a compiler warning if not all cases are covered. To migrate, ensure you have a case for every enum variant or a `default` fallback.

### 3. Trait Bounds

Generic functions that specify trait bounds (e.g., `<T: Add>`) will now strictly reject any types passed to them that do not have a registered `impl` for that trait.

---

## Migrating from Ferrite v2.0 to v2.1

Ferrite v2.1 introduces the standard library and built-ins. While largely additive, it changes how certain core utilities are accessed.

### 1. Standard Library Access

Standard library modules (`math`, `strings`, `collections`, `io`) are now available via the `import` statement.

**v2.0 Approach:**
You had to define your own math constants or string helpers.

**v2.1 Approach:**

```ferrite
import "math";
import "strings";

keep p: float = math.PI;
keep s: string = strings.upper("ferrite");
```

### 2. Initialization Helpers

The `init()` stub for tensors has been renamed to `zeros()` to better reflect its behavior and align with ML conventions.

**v2.0:** `param w: Tensor<float, (10)> = init();`
**v2.1:** `param w: Tensor<float, (10)> = zeros();`

### 3. Collection Indexing

You can now use `[]` indexing on `Map` and `List` types (previously reserved for Tensors).

```ferrite
import "collections";
keep m: Map<string, int> = Map { ignore: 0 };
keep val: int = m["key"];
```

---

## Migrating from Ferrite v1.4 to v2.0

Ferrite v2.0 is a **complete rewrite**. The language has changed from a dynamically-typed scripting language to a statically-typed, ahead-of-time compiled ML language. This guide covers every breaking change.

---

## Paradigm Shift

| Aspect           | v1.4.0                    | v2.0.0                             |
| :--------------- | :------------------------ | :--------------------------------- |
| Typing           | Dynamic                   | Static (compile-time)              |
| Execution        | Bytecode VM (interpreter) | AOT compiled (LLVM native)         |
| Type annotations | None                      | Required on all declarations       |
| Null             | `null` value exists       | No null — use `enum Option<T>`     |
| Error handling   | `try/catch/throw`         | Compile-time errors only (for now) |
| REPL             | Interactive shell         | Not available                      |

---

## Keyword Changes

| v1.4 Keyword | v2.0 Keyword | Notes                           |
| :----------- | :----------- | :------------------------------ |
| `let`        | `keep`       | Requires type annotation        |
| `fn`         | `fun`        | Requires typed parameters       |
| `break`      | `stop`       | Same semantics                  |
| `continue`   | `skip`       | Same semantics                  |
| `else if`    | `elif`       | Single keyword                  |
| _(new)_      | `param`      | Trainable parameter declaration |
| _(new)_      | `constant`   | Compile-time constant           |
| _(new)_      | `group`      | Struct-like type declaration    |
| _(new)_      | `enum`       | Algebraic data type             |
| _(new)_      | `infer`      | Inference execution context     |
| _(new)_      | `train`      | Training execution context      |
| _(new)_      | `where`      | Type/shape constraints          |

---

## Variable Declarations

### Before (v1.4)

```ferrite
let x = 42;
let name = "ferrite";
```

### After (v2.0)

```ferrite
keep x: int = 42;
keep name: string = "ferrite";
```

**Every variable must have an explicit type annotation.** Dynamic typing is gone.

---

## Functions

### Before (v1.4)

```ferrite
fn add(a, b) {
    return a + b;
}

fn greet(name, ...titles) {
    return "Hello " + name;
}
```

### After (v2.0)

```ferrite
fun add(a: int, b: int) -> int {
    return a + b;
}

// Variadic functions are NOT supported in v2.0
```

---

## Control Flow

### Before (v1.4)

```ferrite
if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else {
    "F"
}

while running { break; }
for item in list { continue; }
```

### After (v2.0)

```ferrite
if score >= 90 {
    keep grade: string = "A";
} elif score >= 80 {
    keep grade: string = "B";
} else {
    keep grade: string = "F";
}

while running { stop; }
for item in list { skip; }
```

---

## Removed Features

### F-Strings

```ferrite
// v1.4: let msg = f"Hello {name}";
// v2.0: No f-strings. Use explicit concatenation or formatting functions.
```

### Null

```ferrite
// v1.4: let x = null;
// v2.0: No null. Use enum Option<T> { Some(T); None; }
```

### Try/Catch/Throw

```ferrite
// v1.4:
// try { risky(); } catch err { handle(err); }
// v2.0: Not available. Errors are compile-time only.
```

### Destructuring

```ferrite
// v1.4: let [a, b, ...rest] = [1, 2, 3, 4];
// v2.0: Not available. Use explicit indexing.
```

### Null Coalescing

```ferrite
// v1.4: let port = config["port"] ?? 8080;
// v2.0: Not available. No null means no need for ??.
```

### Maps/Dicts

```ferrite
// v1.4: let user = {"name": "Alice", "age": 30};
// v2.0: Use group types instead:
group User {
    name: string;
    age: int;
}
keep user: User = User { name: "Alice", age: 30 };
```

### REPL

The interactive REPL is not available in v2.0. Use `ferrite check file.fe` to validate code.

---

## New Features in v2.0

### Tensor Types

```ferrite
param weights: Tensor<float, (784, 128)> = init();
```

### Generics & Trait Bounds

```ferrite
fun identity<T>(x: T) -> T { return x; }
fun bounded<T: Add + Mul>(a: T, b: T) -> T { return a; }
```

### Groups (Structs)

```ferrite
group Vector {
    x: float;
    y: float;
    fun length(self) -> float { return self.x; }
}
```

### ML Blocks

```ferrite
infer { keep output: int = predict(input); }
train { keep loss: float = compute_loss(); }
```

### Pattern Matching (Enhanced)

```ferrite
match value {
    case Some(x) => { process(x); }
    case None => { handle_empty(); }
    default => { fallback(); }
}
```

---

## CLI Changes

| v1.4 Command            | v2.0 Command                |
| :---------------------- | :-------------------------- |
| `ferrite script.fe`     | `ferrite check script.fe`   |
| `ferrite` (starts REPL) | Not available               |
| _(N/A)_                 | `ferrite compile script.fe` |

---

## Accessing v1.4

The v1.4 codebase is preserved on the `v1-legacy` branch:

```bash
git checkout v1-legacy
cargo build --release
./target/release/ferrite examples.fe
```
