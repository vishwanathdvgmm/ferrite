# Migrating from Ferrite v1.4 to v2.0

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
