# Changelog

All notable changes to Ferrite are documented here.

## [2.3.1] — 2026-07-03

### Fixed

- Fixed interpreter crash when using negative list or string indices (now emits a clean runtime error).
- Fixed interpreter missing zero-checks for float division and modulo.
- Fixed `ferrite run` not supporting files with only top-level statements (it now runs them automatically if no `main` is found).
- Fixed method dispatch inside the interpreter failing to inject `self` properly.
- Removed unused mutable variables and unused variables warning during compilation.

### Added

- Documented pure snapshot semantics for closures in interpreter.
- Added comprehensive cross-language benchmark suite (`tests/benchmarks`).
- Upgraded `run_tests.sh` to fully verify interpreter execution (`run`) in addition to semantic type-checking (`check`), and added runtime fail tests.

## [2.3.0] — 2026-06-26

### 🔄 Closures, Guards & Interpreter Control Flow

Ferrite v2.3.0 brings significant enhancements to the tree-walk interpreter, including full support for control flow inside loops, advanced pattern matching, and proper lexical closures.

### Added

- **Closures & Lambda Capture** — Lambdas `(params) => expr` now correctly capture their lexical environment at creation time, allowing stateful functions and callbacks.
- **Match Guards** — `case pattern if cond =>` syntax added for conditional pattern matching.
- **Advanced Destructuring** — `match` statements now fully support deep enum and struct destructuring in the interpreter.
- **Loop Control Flow** — `stop` (break) and `skip` (continue) are now fully operational in the tree-walk interpreter for `while` and `for` loops.
- **String Concatenation** — The `+` operator now properly supports `string + string` concatenation in the semantic analyzer.
- **3 New Tests** — Coverage expanded for closures, match guards, and loop control flow.

---

## [2.2.1] — 2026-06-11

### 🛠️ Type System Hardening & Traits

Ferrite v2.2.1 introduces the core architectural shift necessary for robust code reusability: Trait definitions, `impl` blocks, and operator overloading via trait dispatch.

### 🚀 Tree-Walk Interpreter (`ferrite run`)

Because compiling Ferrite ahead-of-time to native binaries requires complex LLVM 15 C++ development libraries, we've introduced a built-in pure-Rust tree-walk interpreter! You can now execute `.fe` scripts immediately without needing a local C++ toolchain via `ferrite run <file.fe>`.

### Added

- **`ferrite run` Interpreter** — Evaluates the checked AST directly in a pure Rust runtime environment.
- **`impl` blocks** — Standalone implementation blocks for both traits (`impl Add for Point`) and inherent methods (`impl Point`).
- **Operator Overloading** — `+`, `-`, `*`, `/`, `%` now automatically dispatch to `Add`, `Sub`, `Mul`, `Div`, `Mod` traits respectively for user-defined types.
- **Match Exhaustiveness** — The semantic analyzer now issues warnings for `match` statements on `enum` types that do not cover all variants.
- **`Self` type resolution** — The `Self` keyword is now semantically resolved within `trait` and `impl` contexts.
- **Enum Constructors** — Enum variants are now properly registered as callable functions (e.g. `Some(42)` correctly infers to `Option<int>`).
- **Proper Field Access** — `p.x` now perfectly infers the field's type via the new group field registry instead of erroring.
- **7 New Tests** — 32 test suite total, covering all trait, bound violation, exhaustiveness, and field access checks.

---

## [2.1.0] — 2026-04-08

### 📚 Standard Library & Compiled Built-ins

Ferrite v2.1 re-introduces the standard library and core built-in functions, now correctly integrated into the AOT compiler pipeline via embedded asset bundling and refined type unification.

### Added

- **Embedded Standard Library** — `math`, `strings`, **collections** (`List`, `Map`), and `io` modules are now built into the binary.
- **Improved Import Resolution** — `import "name"` now resolves both from the filesystem and embedded standard library assets.
- **Refined Type Unification** — Support for `GenericInst` vs `Named` matches, allowing `List<int>` to be initialized by `List { ... }` group literals.
- **Tracked Generic substitutions** — Properly verifies call-site type consistency for generic functions like `push<T>(l: List<T>, item: T)`.
- **Collection Indexing** — Native support for `m[key]` indexing for `Map<K, V>` and `List<T>` types in the semantic analyzer.
- **Expanded Built-ins** — `print`, `println`, `input`, `len`, `str`, `int`, `float`, `assert`, `exit`, and `zeros`.
- **25-Test Suite** — Expanded verification covering built-ins, stdlib imports, and argument arity checking.

### Changed

- **Binary Distribution** — The release binary is now named simply `ferrite.exe` for easier system PATH integration.
- **`init()` → `zeros()`** — Consistent naming for tensor zero-initialization stub.
- **`ImportResolver`** — Now uses an internal virtual path system `<stdlib::name>` to prevent collisions with user files.

---

## [2.0.0] — 2026-04-02

### 🚀 Complete Rewrite: AOT Compiled ML Language

Ferrite v2.0 is a ground-up rewrite from a dynamically-typed bytecode VM interpreter to a statically-typed, ahead-of-time compiled ML programming language.

### Added

- **Static Type System** — `int`, `float`, `bool`, `string`, `Tensor<T, shape>`, generics, `Unit`, `Never`
- **Tensor Types** — `Tensor<float, (784, 128)>` with compile-time shape validation
- **Structural Unification** — strict `unify(expected, actual)` with zero implicit coercion
- **Semantic Analyzer** — two-pass AST walker: forward declarations + full type checking
- **Effect System** — `infer`, `train`, `async` effect annotations on functions
- **ML Blocks** — `infer { }` and `train { }` execution context blocks
- **`keep` / `param`** — typed variable declarations replacing `let`
- **Groups** — struct-like types with fields and methods (`group Point { x: float; }`)
- **Enums (ADTs)** — algebraic data types (`enum Option<T> { Some(T); None; }`)
- **Generics** — type parameters, trait bounds (`T: Add + Mul`), shape parameters
- **Where Clauses** — constraint expressions (`where N > 0, T: Serialize`)
- **Pattern Matching** — `match` with literal, wildcard, binding, constructor, struct patterns
- **LLVM Codegen** — native code emission via `inkwell` (behind `llvm` feature flag)
- **22-Test Suite** — 10 pass tests + 12 fail tests with automated runner
- **ANSI Diagnostics** — colored error output with source line display and carets

### Changed

- **`fn` → `fun`** for function declarations
- **`let` → `keep`** for local variable declarations
- **`break` → `stop`**, **`continue` → `skip`** for loop control
- **`else if` → `elif`** for chained conditionals
- **All variables require type annotations** — `keep x: int = 5;`

### Removed

- Bytecode VM (`src/runtime/vm.rs`, `src/codegen/compiler.rs`, `src/codegen/opcodes.rs`)
- Dynamic typing — no `null`, no truthiness, no runtime type checks
- F-strings and string interpolation
- Try/catch/throw error handling
- Variadic functions (`...args`)
- List and map destructuring (`let [a, ...rest] = list`)
- Null coalescing (`??`)
- REPL interactive mode
- Built-in functions (`print`, `len`, `map`, `filter`, etc.) — stdlib migration pending

---

## [1.4.0] — 2026-03-15

### Architecture: Bytecode VM

Complete transition from tree-walking interpreter to stack-based bytecode VM.

### Added

- Bytecode compiler and stack-based VM
- Semantic resolver for static variable resolution
- Embedded standard library (no external files needed)
- `mathutils`, `strings`, `collections`, `functional` modules
- F-strings (`f"Hello {name}"`)
- Try/catch/throw error handling
- File I/O (`read_file`, `write_file`, `append_file`, `file_exists`)
- Variadic functions (`fn log(level, ...messages)`)
- Null coalescing (`??`)
- Stateful closures via `Rc<RefCell<HashMap>>`
- ~50 built-in functions

---

## [1.0.0] — 2026-02-15

### Initial Release

Single-file tree-walking interpreter.

### Added

- Dynamically-typed scripting language
- Recursive descent parser with Pratt expression parsing
- `if`/`else if`/`else`, `while`, `for`, `match` control flow
- First-class functions and closures
- Lists, maps, and destructuring
- ~30 built-in functions
- Interactive REPL
