# 🦀 Ferrite

A statically-typed, ahead-of-time compiled ML programming language — built in Rust.

---

## 🚀 Quick Start (v2.0)

```bash
# Build the compiler
cargo build --release

# Type-check a Ferrite program
./target/release/ferrite check program.fe

# Compile to native LLVM IR (requires --features llvm)
cargo build --release --features llvm
./target/release/ferrite compile program.fe
```

---

## 📖 Language Tour

### Variables & Types

```ferrite
keep x: int = 42;
keep name: string = "Ferrite";
keep pi: float = 3.14159;
keep flag: bool = true;
```

All variables require explicit type annotations. There is no `null`, no dynamic typing, and no implicit coercion.

### Functions

```ferrite
fun add(a: int, b: int) -> int {
    return a + b;
}

fun greet(name: string) {
    // Returns Unit (no return type annotation)
}
```

### Control Flow

```ferrite
if score > 90 {
    keep grade: string = "A";
} elif score > 80 {
    keep grade: string = "B";
} else {
    keep grade: string = "F";
}

keep i: int = 0;
while i < 10 {
    if i == 5 { stop; }   // break
    if i == 3 { skip; }   // continue
    i = i + 1;
}
```

### Groups (Structs)

```ferrite
group Point {
    x: float;
    y: float;

    fun distance(self) -> float {
        return self.x;
    }
}

keep p: Point = Point { x: 1.0, y: 2.0 };
```

### Enums & Pattern Matching

```ferrite
enum Option<T> {
    Some(T);
    None;
}

match value {
    case 0 => { return "zero"; }
    case 1 => { return "one"; }
    default => { return "other"; }
}
```

### Tensor Types

```ferrite
param weights: Tensor<float, (784, 128)> = init();
param bias: Tensor<float, (128)> = init();
// Symbolic dimensions for batch processing
param input: Tensor<float, (B, 784)> = fetch();
```

Shape mismatches are caught at compile time. No implicit broadcasting or reshaping.

### ML Blocks & Effects

```ferrite
infer fun predict(x: int) -> int {
    return x;
}

train {
    keep loss: float = compute_loss();
}
```

### Generics & Trait Bounds

```ferrite
fun identity<T>(x: T) -> T {
    return x;
}

fun bounded<T: Add + Mul>(a: T, b: T) -> T {
    return a;
}

fun constrained<N: shape>(size: int) -> int
    where N > 0 {
    return size;
}
```

### Constants & Imports

```ferrite
constant PI: float = 3.14159;
constant MAX_EPOCHS: int = 100;

import "module_path";
from "path" take function_name;
```

---

## 🎯 Complete Showcase — All v2.0 Features

Save this as `showcase.fe` and run `ferrite check showcase.fe`:

```ferrite
// ── Top-level Constants ─────────────────────────────────────
constant PI: float = 3.14159;
constant MAX_EPOCHS: int = 100;
constant MODEL_NAME: string = "FerriteMLP";
constant DEBUG: bool = false;

// ── Enums (Algebraic Data Types) ────────────────────────────
enum Color {
    Red;
    Green;
    Blue;
}

enum Option<T> {
    Some(T);
    None;
}

// ── Groups (Structs) with Methods ───────────────────────────
group Vector2 {
    x: float;
    y: float;

    fun magnitude(self) -> float {
        return self.x;
    }

    fun scale(self, factor: float) -> float {
        return self.x;
    }
}

group Layer {
    units: int;
    activation: string;
}

// ── Generic Functions with Trait Bounds ──────────────────────
fun identity<T>(x: T) -> T {
    return x;
}

fun constrained<T: Add + Mul>(a: T, b: T) -> T {
    return a;
}

fun with_shape<N: shape>(dim: int) -> int
    where N > 0 {
    return dim;
}

// ── Effect-Annotated Functions (ML Contexts) ────────────────
infer fun predict(input: int) -> int {
    return input * 2;
}

train fun optimize(loss: float) -> float {
    return loss * 0.99;
}

// ── Tensor Types with Shape Validation ──────────────────────
fun zeros() -> int {
    return 0;
}

fun build_model() {
    param weights: Tensor<float, (784, 128)> = zeros();
    param bias: Tensor<float, (128)> = zeros();
    param kernel: Tensor<float, (3, 3, 64)> = zeros();
    param embeddings: Tensor<float, (B, 768)> = zeros();
}

// ── Full Expression Coverage ────────────────────────────────
fun expressions_demo() {
    keep a: int = 10;
    keep b: int = 20;

    // Arithmetic
    keep sum: int = a + b;
    keep diff: int = a - b;
    keep prod: int = a * b;
    keep quot: int = a / b;
    keep rem: int = a % b;

    // Unary
    keep neg: int = -a;
    keep flag: bool = true;
    keep flipped: bool = !flag;

    // Comparison
    keep lt: bool = a < b;
    keep gt: bool = a > b;
    keep eq: bool = a == b;
    keep ne: bool = a != b;

    // Logical
    keep both: bool = flag && !flag;
    keep either: bool = flag || !flag;

    // Float
    keep radius: float = 5.0;
    keep area: float = PI * radius * radius;
}

// ── Control Flow ────────────────────────────────────────────
fun classify(score: int) -> string {
    if score > 90 {
        return "excellent";
    } elif score > 70 {
        return "good";
    } elif score > 50 {
        return "average";
    } else {
        return "needs improvement";
    }
}

fun countdown(n: int) -> int {
    keep i: int = n;
    while i > 0 {
        if i == 42 { stop; }   // break
        i = i - 1;
    }
    return i;
}

fun loop_control() {
    keep i: int = 0;
    while i < 100 {
        i = i + 1;
        if i % 2 == 0 { skip; }  // continue
    }
}

// ── Pattern Matching ────────────────────────────────────────
fun describe(code: int) -> string {
    match code {
        case 200 => { return "OK"; }
        case 404 => { return "Not Found"; }
        case 500 => { return "Server Error"; }
        default  => { return "Unknown"; }
    }
}

// ── ML Blocks (Infer / Train) ───────────────────────────────
fun training_loop() {
    train {
        keep lr: float = 0.001;
        keep momentum: float = 0.9;
    }
    infer {
        keep prediction: int = predict(10);
    }
}

// ── Group Literals & Field Access ───────────────────────────
fun create_objects() {
    keep origin: Vector2 = Vector2 { x: 0.0, y: 0.0 };
    keep point: Vector2 = Vector2 { x: 3.0, y: 4.0 };
    keep fc1: Layer = Layer { units: 128, activation: "relu" };

    keep mag: float = point.magnitude();
    keep scaled: float = point.scale(2.5);
}

// ── Scoping Rules ───────────────────────────────────────────
fun scoping_demo() {
    keep x: int = 1;
    if true {
        keep x: int = 2;         // Shadowing — allowed
        keep inner: int = x + 1;
    }
    keep result: int = x;        // x is still 1
}

// ── Entry Point ─────────────────────────────────────────────
fun main() -> int {
    keep grade: string = classify(85);
    keep status: string = describe(200);
    keep count: int = countdown(10);

    expressions_demo();
    loop_control();
    training_loop();
    create_objects();
    scoping_demo();
    build_model();

    return 0;
}
```

**Expected output:**

```
$ ferrite check showcase.fe
✅ Type-checking successful.
```

The compiler verifies all types, scoping rules, operator constraints, and ML invariants at compile time — if it passes, the code is guaranteed to be type-safe.

---

## 🏗️ Compiler Architecture

```
Source (.fe) → Lexer → Parser → ImportResolver → TypeEnv → SemanticAnalyzer → LLVM Codegen
```

```
ferrite/
├── src/
│   ├── main.rs          # CLI driver (check / compile)
│   ├── ast/             # AST node definitions
│   ├── codegen/         # LLVM IR emission (feature-gated)
│   ├── errors/          # Span, Diagnostic, DiagnosticBag
│   ├── imports/         # Module resolution with cycle detection
│   ├── lexer/           # UTF-8 tokenizer (34 keywords)
│   ├── parser/          # Recursive descent with panic-mode recovery
│   ├── semantic/        # Two-pass type-checking AST walker
│   └── types/           # Static type system & tensor shape validation
├── tests/               # 22-test verification suite
├── docs/                # Language documentation
├── ARCHITECTURE.md      # Detailed compiler architecture
├── CHANGELOG.md         # Version history
└── MIGRATION.md         # v1.4 → v2.0 migration guide
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for a detailed breakdown of each compiler phase.

---

## 🧪 Testing

```bash
# Run the full 22-test verification suite
bash tests/run_tests.sh
```

The test suite includes:
- **10 pass tests**: primitives, functions, control flow, groups, enums, constants, generics, tensors, ML blocks, expressions
- **12 fail tests**: type mismatches, undefined variables, return errors, scope violations, syntax errors, operator type errors

---

## 💡 Design Principles

- **ML-First** — Tensor types, training/inference effects, and shape validation are built into the language core
- **Strict Typing** — Zero implicit coercion, zero broadcasting, zero runtime reflection
- **AOT Compiled** — Targets native code via LLVM; no interpreter, no VM
- **Portable Frontend** — The compiler frontend builds on any Rust target without requiring LLVM installed
- **Pure Safe Rust** — No `unsafe` code in the compiler

---

## 📚 Documentation

| Document                                        | Description                          |
|:------------------------------------------------|:-------------------------------------|
| [docs/syntax.md](docs/syntax.md)                | Language syntax reference            |
| [docs/semantics.md](docs/semantics.md)          | Compiler pipeline & semantics        |
| [docs/type-system.md](docs/type-system.md)      | Static type system specification     |
| [docs/grammar.ebnf](docs/grammar.ebnf)          | Formal EBNF grammar                  |
| [ARCHITECTURE.md](ARCHITECTURE.md)              | Compiler architecture                |
| [MIGRATION.md](MIGRATION.md)                    | v1.4 → v2.0 migration guide         |
| [CHANGELOG.md](CHANGELOG.md)                    | Version history                      |

---

## 📦 Releases

| Version | Tag       | Description                            |
|:--------|:----------|:---------------------------------------|
| v2.0.0  | `v2.0.0`  | AOT compiled ML language               |
| v1.4.0  | `v1.4.0-final` | Bytecode VM (on `v1-legacy` branch) |
| v1.0.0  | `v1.0.0`  | Initial tree-walking interpreter        |
