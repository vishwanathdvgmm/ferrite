# 🦀 Ferrite (v3.2.0)

> **The AI-Native, Statically-Typed Systems & ML Programming Language — Built in Rust.**

[![Website](https://img.shields.io/badge/Website-ferrite--lang.org-326efa)](https://www.ferrite-lang.org/)
[![Documentation](https://img.shields.io/badge/Docs-v3.2.0-28285a)](https://www.ferrite-lang.org/docs/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Sponsor](https://img.shields.io/badge/Sponsor-GitHub%20Sponsors-d9534f)](https://www.ferrite-lang.org/sponsors/)

---

## 🌟 Overview

Ferrite is an ahead-of-time compiled and interpreted programming language engineered specifically for artificial intelligence, machine learning, and systems programming. Built from the ground up in pure safe Rust, Ferrite provides:

- **Native Multidimensional Tensors**: Compile-time shape verification (`Tensor<float, (1, 4)>`) with native matrix multiplication (`@`), and comprehensive builtins (`rand`, `ones`, `zeros`).
- **Native Collections (DSA)**: Rich, dynamic data structures including `List<T>` and `Map<K, V>` with robust string manipulation.
- **Expression-Oriented Architecture**: `if`, `match`, and blocks `{}` evaluate to values, with the `Never` type ensuring seamless control flow typing (`stop`, `skip`, `return`).
- **Zero Implicit Coercion**: Strict type safety—no implicit numeric promotion or dynamic truthiness.
- **Scope-Delimited Execution Contexts**: `train { ... }` (autodiff gradient tracking) vs. `infer { ... }` (zero-overhead forward pass).
- **Module System**: `import "math"`, destructuring imports (`from "utils" take { helper as my_helper }`), and `pub` visibility.
- **Dual Execution Model**: Instant execution via the built-in Tree-Walk Interpreter or native binary compilation via LLVM AOT.
- **VS Code Support**: Official IDE extension with semantic highlighting, auto-formatting, and Language Server (LSP) integration.

---

## 🚀 Quick Start (v3.2.0)

### Installation

```bash
# Windows Setup Wizard or Portable Binary
Download ferrite-v3.2.0-setup.exe from Releases

# macOS (Homebrew)
brew tap vishwanathdvgmm/tap
brew install ferrite

# Linux / Unix
curl -fsSL https://ferrite-lang.org/install.sh | sh

# VS Code Extension
Search for "Ferrite Programming Language" in the VS Code Marketplace or Open VSX Registry and install the official extension (v1.3.1+) for full IDE support with smart compiler discovery!
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

## ❤️ Support & Sponsorship

Ferrite is 100% open-source software maintained by Vishwanath M M and contributors. If you find Ferrite valuable, consider supporting development:

- ☕ **[Sponsor on GitHub Sponsors](https://github.com/sponsors/vishwanathdvgmm)**
- 💎 **[View Sponsor Tiers on our Website](https://www.ferrite-lang.org/sponsors/)**

---

## 📜 License

Ferrite is licensed under the [MIT License](LICENSE) and [Apache License 2.0](LICENSE).
