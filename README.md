# 🦀 Ferrite (v2.4.1)

> **The AI-Native, Statically-Typed Systems & ML Programming Language — Built in Rust.**

[![Website](https://img.shields.io/badge/Website-ferrite--lang.org-326efa)](https://www.ferrite-lang.org/)
[![Documentation](https://img.shields.io/badge/Docs-v2.4.1-28285a)](https://www.ferrite-lang.org/docs/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Sponsor](https://img.shields.io/badge/Sponsor-GitHub%20Sponsors-d9534f)](https://www.ferrite-lang.org/sponsors/)

---

## 🌟 Overview

Ferrite is an ahead-of-time compiled and interpreted programming language engineered specifically for artificial intelligence, machine learning, and systems programming. Built from the ground up in pure safe Rust, Ferrite provides:

- **Native Multidimensional Tensors**: Compile-time shape verification (`Tensor<float, (100, 784)>`) with matrix multiplication (`@`).
- **Zero Implicit Coercion**: Strict type safety—no implicit numeric promotion or dynamic truthiness.
- **Scope-Delimited Execution Contexts**: `train { ... }` (autodiff gradient tracking) vs. `infer { ... }` (zero-overhead forward pass).
- **Module System**: `import "math"`, destructuring imports (`from "utils" take { helper as my_helper }`), and `pub` visibility.
- **Dual Execution Model**: Instant execution via the built-in Tree-Walk Interpreter or native binary compilation via LLVM AOT.

---

## 🚀 Quick Start (v2.4.1)

### Installation

```bash
# Windows Setup Wizard or Portable Binary
Download ferrite-v2.4.0-setup.exe from Releases

# macOS (Homebrew)
brew tap vishwanathdvgmm/tap
brew install ferrite

# Linux / Unix
curl -fsSL https://ferrite-lang.org/install.sh | sh
```

### Usage

```bash
# Parse & Type-check without execution
ferrite check my_code.fe

# Interpret script immediately (Tree-Walk Engine)
ferrite run my_code.fe

# Compile to native LLVM binary (AOT Mode)
ferrite compile my_code.fe
```

---

## 📖 Language Tour

### Variables & Immutability (`keep`)

```ferrite
// Variables are immutable by default
keep pi: float = 3.14159;
keep name: string = "Ferrite";

// Reassignable mutable variables
keep count: int = 0;
count = count + 1;
```

### Functions & Closures

```ferrite
fun add(a: int, b: int) -> int {
    return a + b;
}

// Anonymous closures capturing outer scope
keep factor = 2;
keep double = (x: int) => x * factor;
```

### Module Import System (v2.4.0+)

```ferrite
// Exporting in math.fe
pub fun square(n: int) -> int {
    return n * n;
}

// Importing in main.fe
import "math";
from "utils" take { format as fmt };

keep result = math.square(5);
```

### Control Flow & Guards

```ferrite
if score > 90 {
    println("Grade: A");
} else {
    println("Grade: B");
}

keep i: int = 0;
while i < 10 {
    i = i + 1;
    if i == 3 { skip; } // continue loop
    if i == 8 { stop; } // break loop
}
```

### Pattern Matching on Enums

```ferrite
enum Result<T> {
    Ok(T);
    Err(string);
}

keep status = Ok(200);

match status {
    case Ok(code) if code == 200 => println("Success 200");
    case Ok(code) => println("Status: " + str(code));
    case Err(msg) => println("Error: " + msg);
}
```

### Native Shaped Tensors & Matrix Operations

```ferrite
import "math";

// Validated matrix dimensions: (100, 784) x (784, 10) => (100, 10)
param inputs: Tensor<float, (100, 784)> = ones();
param weights: Tensor<float, (784, 10)> = rand();

infer {
    keep logits = inputs @ weights; // Checked at compile-time!
    println("Output computed cleanly.");
}
```

### Groups (Structs) & Traits

```ferrite
group Point {
    x: float;
    y: float;
}

trait Display {
    fun format(self) -> string;
}

impl Display for Point {
    fun format(self) -> string {
        return "Point(" + str(self.x) + ", " + str(self.y) + ")";
    }
}
```

---

## 🏗️ Compiler Architecture & EKB

The Ferrite toolchain pipeline:

```
Source (.fe) → Lexer → Parser → SymbolResolver → SemanticAnalyzer (Shape Unification) → Interpreter / LLVM Codegen
```

For compiler engineers and contributors, we have published the **21-Chapter Engineering Knowledge Base (EKB)** specification detailing compiler internals, AST unification, and codegen:

📖 **[Explore the 21-Chapter EKB Architecture Guide](https://github.com/vishwanathdvgmm/ferrite/tree/main/documentation)**

---

## 📚 Documentation & Resources

- 🌐 **Website**: [ferrite-lang.org](https://www.ferrite-lang.org/)
- 📖 **Language Documentation**: [ferrite-lang.org/docs/](https://www.ferrite-lang.org/docs/)
- 🎓 **Interactive Tutorial**: [ferrite-lang.org/tutorial/](https://www.ferrite-lang.org/tutorial/)
- 🕹️ **Web Playground**: [ferrite-lang.org/playground/](https://www.ferrite-lang.org/playground/)
- 📄 **Wikipedia Entry**: [`WIKIPEDIA.md`](WIKIPEDIA.md) / [Wikipedia Web Page](https://www.ferrite-lang.org/wikipedia/)
- 🌐 **Community Hub**: [ferrite-lang.org/community/](https://www.ferrite-lang.org/community/)
- ❤️ **Sponsor Ferrite**: [ferrite-lang.org/sponsors/](https://www.ferrite-lang.org/sponsors/)

---

## 📦 Version History

| Version    | Release Date | Key Features & Highlights                                        |
| :--------- | :----------- | :--------------------------------------------------------------- |
| **v2.4.1** | July 2026    | First-Class Closures, 21-Chapter EKB Specification Published     |
| **v2.4.0** | July 2026    | Full Module System (`import`, `from ... take`, `pub` visibility) |
| **v2.3.1** | June 2026    | Performance Optimizations & AST Interpreter Hardening            |
| **v2.3.0** | June 2026    | Guard Clauses, Interpreter Flow Jump Controls                    |
| **v2.2.0** | May 2026     | Generics, Trait Interfaces (`trait`, `impl`)                     |
| **v2.0.0** | May 2026     | Shaped Tensor Generics (`Tensor<T, Shape>`) & Autodiff           |
| **v1.4.0** | March 2026   | Nominal Struct Groups (`group`) & Pattern Matching (`match`)     |
| **v1.0.0** | January 2026 | Initial Release: Core Lexer, Parser, & Tree-Walk Engine          |

---

## ❤️ Support & Sponsorship

Ferrite is 100% open-source software maintained by Vishwanath M M and contributors. If you find Ferrite valuable, consider supporting development:

- ☕ **[Sponsor on GitHub Sponsors](https://github.com/sponsors/vishwanathdvgmm)**
- 💎 **[View Sponsor Tiers on our Website](https://www.ferrite-lang.org/sponsors/)**

---

## 📜 License

Ferrite is licensed under the [MIT License](LICENSE) and [Apache License 2.0](LICENSE).
