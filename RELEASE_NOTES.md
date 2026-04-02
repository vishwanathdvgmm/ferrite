# Ferrite v2.0.0 Release Notes

Welcome to **Ferrite v2.0.0** 🚀 — A complete ground-up rewrite from a dynamically-typed bytecode VM interpreter to a **statically-typed, ahead-of-time compiled ML programming language**.

---

## 🏗️ What Changed: Everything

Ferrite v2.0 is not an incremental update — it is a fundamentally new language architecture. The dynamically-typed scripting engine from v1.4 has been completely replaced with a strict, ML-first compiler pipeline targeting native code via LLVM.

### New Compiler Pipeline

```
Source (.fe) → Lexer → Parser → ImportResolver → TypeEnv → SemanticAnalyzer → LLVM Codegen
```

Every stage is a clean, independent Rust module:

| Stage                 | Module                | What it does                                                                                                                                  |
| :-------------------- | :-------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------- |
| **Lexer**             | `src/lexer/`          | Scans UTF-8 source into span-annotated tokens. Recognizes 34 keywords including `keep`, `param`, `infer`, `train`, `group`, `enum`            |
| **Parser**            | `src/parser/`         | Recursive descent parser (~1300 lines) with panic-mode error recovery. Produces a strongly-typed AST with 34+ node types                      |
| **Import Resolver**   | `src/imports/`        | DAG-based module resolution with circular dependency detection. Caches parsed ASTs to prevent re-parsing                                      |
| **Type Environment**  | `src/types/`          | Maps AST types to resolved `Type` values. Manages scoped symbol tables (stack of HashMaps). Performs structural unification                   |
| **Tensor Shapes**     | `src/types/tensor.rs` | Validates tensor dimension matching. Supports constant (`784`) and symbolic (`B`) dimensions. **No implicit broadcasting or reshaping**       |
| **Semantic Analyzer** | `src/semantic/`       | Two-pass AST walker. Pass 1: forward-declares all types, constants, and functions. Pass 2: full recursive type checking with scope management |
| **LLVM Codegen**      | `src/codegen/`        | Emits native LLVM IR via `inkwell` (LLVM 15 bindings). Feature-gated behind `--features llvm` so the frontend compiles without LLVM installed |

### Deleted Components

The following v1.4 components have been completely removed:

| Component           | v1.4 File                     | Status     |
| :------------------ | :---------------------------- | :--------- |
| Bytecode Compiler   | `src/codegen/compiler.rs`     | ❌ Deleted |
| Opcode Definitions  | `src/codegen/opcodes.rs`      | ❌ Deleted |
| Stack-based VM      | `src/runtime/vm.rs`           | ❌ Deleted |
| Tree-walking Interp | `src/runtime/mod.rs` (interp) | ❌ Deleted |

---

## ✨ New Language Features

### Static Type System

Every variable, parameter, and return type must be explicitly annotated. The compiler enforces strict structural type equality with **zero implicit coercion**.

```ferrite
keep x: int = 42;
keep y: float = 3.14;
// keep z: int = 3.14;  ← COMPILE ERROR: Type mismatch
```

### Tensor Types with Shape Validation

```ferrite
param weights: Tensor<float, (784, 128)> = init();
param input:   Tensor<float, (B, 784)>   = fetch();
```

- Element types restricted to `int` and `float`
- Shape dimensions can be constants (`784`) or symbolic (`B`)
- Mismatched shapes are compile errors — no implicit broadcasting

### Groups (Struct Types)

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

### Enums (Algebraic Data Types)

```ferrite
enum Option<T> {
    Some(T);
    None;
}

enum Color { Red; Green; Blue; }
```

### Generics, Trait Bounds & Where Clauses

```ferrite
fun identity<T>(x: T) -> T { return x; }

fun bounded<T: Add + Mul>(a: T, b: T) -> T { return a; }

fun shaped<N: shape>(size: int) -> int
    where N > 0 {
    return size;
}
```

### ML Execution Blocks

```ferrite
infer fun predict(x: int) -> int { return x; }

train {
    keep loss: float = compute_loss();
}
```

### Pattern Matching

```ferrite
match value {
    case Some(x) => { process(x); }
    case 0 => { handle_zero(); }
    case _ => { fallback(); }
}
```

Supports: literal patterns, wildcards (`_`), variable bindings, constructor patterns (`Some(x)`), and struct patterns (`Point { x, y }`).

### Enhanced Error Diagnostics

ANSI-colored error output with source context:

```
error: Type mismatch: expected 'int', found 'float'. Implicit coercion is forbidden.
  --> program.fe:3:20
  |
3 |     keep x: int = 3.14;
  |                    ^^^^
```

---

## 🧪 Verification

Ferrite v2.0.0 ships with a rigorous 22-test verification suite:

| Category      | Tests  | Coverage                                                                                                                                                                                                                            |
| :------------ | :----- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pass (10)** | ✅ All | Primitives, functions, control flow, groups, enums, constants, generics, tensors, ML blocks, all expression operators                                                                                                               |
| **Fail (12)** | ✅ All | Type mismatches, undefined variables, return errors, non-bool conditions, stop/skip outside loops, duplicate variables, no coercion, missing semicolons, missing braces, negate string, invalid tensor elements, logic on non-bools |

```bash
bash tests/run_tests.sh
# 🎉 ALL 22 TESTS PASSED — Ferrite v2.0 is verified!
```

---

## 🔧 CLI Usage

```bash
# Type-check only
ferrite check program.fe

# Compile to native LLVM IR (requires --features llvm build)
ferrite compile program.fe
```

---

## 📚 Documentation

- [docs/syntax.md](docs/syntax.md) — Language syntax reference
- [docs/semantics.md](docs/semantics.md) — Compiler pipeline & operational semantics
- [docs/type-system.md](docs/type-system.md) — Static type system specification
- [docs/grammar.ebnf](docs/grammar.ebnf) — Formal EBNF grammar
- [ARCHITECTURE.md](ARCHITECTURE.md) — Full compiler architecture
- [MIGRATION.md](MIGRATION.md) — v1.4 → v2.0 migration guide

---

## ⬆️ Migrating from v1.4

See [MIGRATION.md](MIGRATION.md) for a complete guide. Key changes:

| v1.4                 | v2.0                             |
| :------------------- | :------------------------------- |
| `let x = 42`         | `keep x: int = 42`               |
| `fn add(a, b)`       | `fun add(a: int, b: int) -> int` |
| `break` / `continue` | `stop` / `skip`                  |
| `else if`            | `elif`                           |
| Dynamic typing       | Static typing                    |
| Bytecode VM          | LLVM AOT                         |

The v1.4 codebase is preserved on the [`v1-legacy`](https://github.com/vishwanathdvgmm/ferrite/tree/v1-legacy) branch.

---

# Ferrite v1.4.0 Release Notes

Welcome to **Ferrite v1.4.0**! 🚀

This massive update brings the language from a simple tree-walking interpreter to a high-performance **Bytecode Virtual Machine** architecture. We've introduced a robust, statically-embedded Standard Library, advanced semantic analysis, and stateful closures.

## 🏗️ New Architecture: The Bytecode VM

The biggest change is internal. Ferrite now compiles your scripts into optimized bytecode before execution:

- **Semantic Resolver**: A new pass that validates your code for undefined variables and control flow errors before it even runs.
- **Bytecode Compiler**: Translates the AST into linear, stack-based opcodes.
- **Stack-based VM**: A fast, memory-efficient virtual machine that executes the generated bytecode.

## What's New in v1.4.0?

- 📦 **Embedded Standard Library**
    - **No external files needed!** All standard modules are now statically embedded directly into the Ferrite binary from `src/stdlib/`.
    - **Unified Module Loading**: `import "mathutils"` or `import "std/mathutils"` works automatically.
    - **`mathutils`**: `clamp`, `lerp`, `fibonacci`, `is_prime`, `gcd`, `factorial`.
    - **`strings`**: `repeat_str`, `pad_left`, `capitalize`, `title_case`, `is_numeric`.
    - **`collections`**: `flatten`, `chunk`, `unique`, `group_by`, `any`, `all`.
    - **`functional`**: `compose`, `pipe`, `partial`, `memoize`.

- 🛡️ **Robust Error Handling (`try/catch/throw`)**
    - Scripts no longer panic the Rust runtime. Errors like division by zero or invalid operations are caught and localized.
    - Deep stack propagation: Errors can be thrown from anywhere and caught at any level.
    - Custom exceptions: `throw {"code": 404, "msg": "Not Found"}`.

- ✨ **F-Strings (String Interpolation)**
    - Python-style interpolation: `let msg = f"Hello {name}, you are version {v}";`.

- 📂 **File I/O**
    - Full support for `read_file`, `write_file`, `append_file`, and `file_exists`.

- 🧩 **Advanced Syntax & Unpacking**
    - **Variadic Functions**: `fn log(level, ...messages)`.
    - **Destructuring**: `let [a, b, ...rest] = [1, 2, 3, 4]`.
    - **Object Unpacking**: `let { name, age } = user`.
    - **Null Coalescing**: `let port = config["port"] ?? 8080`.

- 🔄 **Stateful Closures**
    - Functions now capture their environment via `Rc<RefCell<HashMap>>`, allowing counters and complex shared state across calls.

---

# Ferrite v1.0.0 Release Notes

Welcome to the first official stable release of **Ferrite v1.0.0**! 🎉

Ferrite is a clean, expressive scripting language written purely in Rust. Designed to be fast, embeddable, and extremely friendly to write. This binary contains everything you need to execute `.fe` scripts or run the interactive REPL.

## Core Language Features (v1.0.0)

- **Everything is an Expression:** `if/else`, `match`, and blocks return values.
- **Basic Types:** integers (`i64`), floats (`f64`), booleans, strings, lists, and maps (dictionaries).
- **Control Flow:** `if / else if / else`, `while`, `for x in list`, `break`, `continue`.
- **Pattern Matching:** Advanced `match` statement supporting literals, ranges (`1..5`), wildcard `_`, and variables bindings.
- **Functions & Closures:** First-class functions, anonymous lambdas (`fn(x) { ... }`), and lexical scoping.
- **Built-ins:** Extensive standard operators and ~30 built-in math, string, and list functions (`len`, `map`, `filter`, `reduce`, `keys`, `values`).
- **Zero Dependencies:** The entire interpreter, lexer, and parser is contained in a single 1600-line Rust file for maximum portability.
