# 🦀 Ferrite

A statically-typed, ahead-of-time compiled ML programming language — built in Rust.

---

## 🚀 Quick Start (v2.3)

1. Download the `ferrite.exe` from releases.
2. Create a folder named `Ferrite` in your preferred location (e.g., `C:\Ferrite`).
3. Add `ferrite.exe` to that folder.
4. Add that folder to your system `PATH`.
5. Done — use `ferrite` from anywhere:

```bash
# Execute your script directly using the built-in interpreter
ferrite run program.fe

# Or compile to native code (requires LLVM)
ferrite compile program.fe
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
    case Some(x) if x > 0 => { return "positive"; }
    case Some(_) => { return "non-positive"; }
    case None => { return "missing"; }
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

### Generics, Traits & Impl Blocks

```ferrite
trait Add {
    fun add(self, other: Self) -> Self;
}

group Point {
    x: float;
    y: float;
}

impl Add for Point {
    fun add(self, other: Self) -> Self {
        return Point {
            x: self.x + other.x,
            y: self.y + other.y
        };
    }
}

// Operators automatically dispatch to traits
fun test_add() {
    keep p1: Point = Point { x: 1.0, y: 1.0 };
    keep p2: Point = Point { x: 2.0, y: 2.0 };
    keep p3: Point = p1 + p2;
}

// Trait bounds on generic functions
fun bounded<T: Add + Mul>(a: T, b: T) -> T {
    return a + b;
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

## ⚡ Performance & Benchmarks

Ferrite v2.3 includes an **AST-walking interpreter** alongside its AOT compilation mode. While AST interpreters are fundamentally slower than bytecode VMs or native binaries due to deep recursive AST evaluation, Ferrite implements `Rc` reference-counted memory optimizations to ensure respectable evaluation speeds.

Here is a performance comparison running standard computational workloads:

| Benchmark          | Ferrite (Interpreter) | Python (Bytecode) | Node.js (JIT) | Rust (AOT) | Go (AOT)  |
| ------------------ | --------------------- | ----------------- | ------------- | ---------- | --------- |
| **Fibonacci (25)** | `7093 ms`             | `923 ms`          | `593 ms`      | `847 ms`   | `917 ms`  |
| **Loop Sum (10M)** | `1882 ms`             | `762 ms`          | `572 ms`      | `800 ms`   | `865 ms`  |
| **String Concat**  | `468 ms`              | `542 ms`          | `532 ms`      | `692 ms`   | `1022 ms` |

_Note: String concatenation in Ferrite outperforms competitors by natively delegating to Rust's optimized underlying allocators._

---

## 🏗️ Compiler Architecture

```
Source (.fe) → Lexer → Parser → ImportResolver (Asset Bundling) → TypeEnv (Built-ins) → SemanticAnalyzer → LLVM Codegen / Tree-Walk Interpreter
```

```
├── 📁 .github
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
├── 📁 website
├── ⚙️ .gitignore
├── 📝 ARCHITECTURE.md
├── 📝 CHANGELOG.md
├── 📄 CNAME
├── 📝 CODE_OF_CONDUCT.md
├── ⚙️ Cargo.toml
├── 📄 EULA.txt
├── 📄 LICENSE
├── 📝 MIGRATION.md
├── 📝 README.md
├── 📝 RELEASE_NOTES.md
└── 📝 TERMS.md
```

See [ARCHITECTURE.md](https://github.com/vishwanathdvgmm/ferrite/blob/main/ARCHITECTURE.md) for a detailed breakdown of each compiler phase.

---

## 🧪 Testing

The v2.3 test suite includes **35 exhaustive tests**:

- **Pass tests**: primitives, functions, control flow, groups, enums, constants, generics, tensors, ML blocks, expressions, built-ins, stdlib, traits, impl blocks, exhaustive matches, field access, **closures**, **guard clauses**, **interpreter control flow**.
- **Fail tests**: type mismatches, undefined variables, return errors, scope violations, syntax errors, argument count errors, missing trait methods, trait bound violations, undefined traits.

---

## 💡 Design Principles

- **ML-First** — Tensor types, training/inference effects, and shape validation are built into the language core
- **Strict Typing** — Zero implicit coercion, zero broadcasting, zero runtime reflection
- **Dual Execution Modes** — Execute scripts immediately via the built-in pure-Rust **Tree-Walk Interpreter**, or compile them ahead-of-time to native binaries via LLVM.
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
| v2.3.0  | `v2.3.0`       | Closures, Guards, Interpreter Flow  |
| v2.2.1  | `v2.2.1`       | Type System Hardening & Traits      |
| v2.1.0  | `v2.1.0`       | Standard Library & Builtins         |
| v2.0.0  | `v2.0.0`       | AOT compiled ML language            |
| v1.4.0  | `v1.4.0-final` | Bytecode VM (on `v1-legacy` branch) |
| v1.0.0  | `v1.0.0`       | Initial tree-walking interpreter    |
