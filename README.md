# 🦀 Ferrite

A statically-typed, ahead-of-time compiled ML programming language — built in Rust.

---

## 🚀 Quick Start (v2.1)

1. Download the `ferrite.exe` from releases.
2. Create a folder named `Ferrite` in your preferred location (e.g., `C:\Ferrite`).
3. Add `ferrite.exe` to that folder.
4. Add that folder to your system `PATH`.
5. Done — use `ferrite` from anywhere:

```bash
ferrite check program.fe
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

### Functions & Standard Library

```ferrite
import "math";
import "strings";

fun test() {
    keep s: float = math.sin(math.PI / 2.0);
    keep upper: string = strings.upper("ferrite");
    println(upper); // Built-in function
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
param weights: Tensor<float, (784, 128)> = zeros();
param bias: Tensor<float, (128)> = zeros();
// Symbolic dimensions for batch processing
param input: Tensor<float, (B, 784)> = zeros();
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

## 🏗️ Compiler Architecture

```
Source (.fe) → Lexer → Parser → ImportResolver (Asset Bundling) → TypeEnv (Built-ins) → SemanticAnalyzer → LLVM Codegen
```

```
├── 📁 docs
├── 📁 src
│   ├── 📁 ast
│   ├── 📁 codegen
│   ├── 📁 errors
│   ├── 📁 imports
│   ├── 📁 lexer
│   ├── 📁 parser
│   ├── 📁 runtime
│   ├── 📁 semantic
│   ├── 📁 stdlib
│   ├── 📁 types
│   └── 🦀 main.rs
├── 📁 tests
├── ⚙️ .gitignore
├── 📝 ARCHITECTURE.md
├── 📝 CHANGELOG.md
├── ⚙️ Cargo.toml
├── 📝 MIGRATION.md
├── 📝 README.md
└── 📝 RELEASE_NOTES.md
```

See [ARCHITECTURE.md](https://github.com/vishwanathdvgmm/ferrite/blob/main/ARCHITECTURE.md) for a detailed breakdown of each compiler phase.

---

## 🧪 Testing

The v2.1 test suite includes **25 exhaustive tests**:

- **Pass tests**: primitives, functions, control flow, groups, enums, constants, generics, tensors, ML blocks, expressions, **built-ins**, **stdlib**.
- **Fail tests**: type mismatches, undefined variables, return errors, scope violations, syntax errors, **argument count errors**.

---

## 💡 Design Principles

- **ML-First** — Tensor types, training/inference effects, and shape validation are built into the language core
- **Strict Typing** — Zero implicit coercion, zero broadcasting, zero runtime reflection
- **AOT Compiled** — Targets native code via LLVM; no interpreter, no VM
- **Portable Frontend** — The compiler frontend builds on any Rust target without requiring LLVM installed
- **Pure Safe Rust** — No `unsafe` code in the compiler

---

## 📚 Documentation

| Document                                                                                | Description                      |
| :-------------------------------------------------------------------------------------- | :------------------------------- |
| [Syntax](https://github.com/vishwanathdvgmm/ferrite/blob/main/docs/syntax.md)           | Language syntax reference        |
| [Semantics](https://github.com/vishwanathdvgmm/ferrite/blob/main/docs/semantics.md)     | Compiler pipeline & semantics    |
| [Type System](https://github.com/vishwanathdvgmm/ferrite/blob/main/docs/type-system.md) | Static type system specification |
| [Grammar](https://github.com/vishwanathdvgmm/ferrite/blob/main/docs/grammar.ebnf)       | Formal EBNF grammar              |
| [Architecture](https://github.com/vishwanathdvgmm/ferrite/blob/main/ARCHITECTURE.md)    | Compiler architecture            |
| [Release Notes](https://github.com/vishwanathdvgmm/ferrite/blob/main/RELEASE_NOTES.md)  | Version history                  |
| [Migration](https://github.com/vishwanathdvgmm/ferrite/blob/main/MIGRATION.md)          | Upgrade guides                   |
| [Changelog](https://github.com/vishwanathdvgmm/ferrite/blob/main/CHANGELOG.md)          | Timeline of changes              |

---

## 📦 Releases

| Version | Tag            | Description                         |
| :------ | :------------- | :---------------------------------- |
| v2.1.0  | `v2.1.0`       | Standard Library & Builtins         |
| v2.0.0  | `v2.0.0`       | AOT compiled ML language            |
| v1.4.0  | `v1.4.0-final` | Bytecode VM (on `v1-legacy` branch) |
| v1.0.0  | `v1.0.0`       | Initial tree-walking interpreter    |
