# 🦀 Ferrite

A clean, expressive scripting language — written entirely in Rust.

---

## 🚀 Quick Start (v1.4.0)

```bash
# Build
cargo build --release

# Run a file
./target/release/ferrite examples.fe

# Start the Multi-line REPL
./target/release/ferrite
```

---

## 🏗️ Compiler Architecture

As of v1.4.0, Ferrite has transitioned from a single-file interpreter into a structured compiler pipeline.

```text
├── 📁 docs
│   ├── 📄 grammar.ebnf
│   ├── 📝 semantics.md
│   ├── 📝 standard-library.md
│   ├── 📝 syntax.md
│   └── 📝 type-system.md
├── 📁 src
│   ├── 📁 ast
│   │   └── 🦀 mod.rs
│   ├── 📁 codegen
│   ├── 📁 lexer
│   │   └── 🦀 mod.rs
│   ├── 📁 parser
│   │   └── 🦀 mod.rs
│   ├── 📁 runtime
│   │   └── 🦀 mod.rs
│   ├── 📁 semantic
│   ├── 📁 stdlib
│   │   ├── 📄 collections.fe
│   │   ├── 📄 functional.fe
│   │   ├── 📄 mathutils.fe
│   │   ├── 🦀 mod.rs
│   │   └── 📄 strings.fe
│   └── 🦀 main.rs
├── ⚙️ .gitignore
├── ⚙️ Cargo.toml
├── 📝 README.md
└── 📄 examples.fe
```

> **Note on src/stdlib/:**
> As of v1.4.0, you do not need to distribute an external `std/` or `stdlib/` folder alongside the binary. All standard library scripts (such as `mathutils`, `strings`, `collections`) are statically embedded directly into the Rust executable from `src/stdlib/` at compile time via `include_str!`.

---

## 📖 Language Tour

### Variables & F-Strings

```ferrite
let x = 42;
let name = "Ferrite";
let msg = f"Hello {name}, you are version {x / 30.0}!";
let nothing = null;
let default_val = config_port ?? 8080; # Null coalescing
```

### Lists, Maps & Unpacking

```ferrite
let nums = [1, 2, 3, 4, 5];
let user = {"name": "Alice", "age": 30};

# Array and Object Destructuring
let [first, second, ...rest] = nums;
let { name, age } = user;
```

### Control Flow

```ferrite
# if/else as expressions
let grade = if score >= 90 { "A" } else { "B" };

# Match patterns (Literals, Ranges, Bindings)
match grade {
    "A" => print("Excellent!"),
    "B" => print("Good job!"),
    other => print(f"You got {other}")
}
```

### Loops (with Range & Destructuring)

```ferrite
# Standard loops
let i = 0;
while i < 10 { print(i); i += 1; }

# For-loops over ranges and lists
for n in range(0, 10, 2) { print(n); } # 0, 2, 4, 6, 8

# Enumerate and Zip Destructuring
for [i, fruit] in enumerate(["apple", "banana"]) {
    print(f"{i}: {fruit}");
}
```

### Functions & Closures

```ferrite
# Variadic functions
fn log_msg(level, ...messages) {
    print(level + ": " + join(messages, " "));
}
log_msg("INFO", "Server", "started.");

# Mutable closures
fn make_counter() {
    let count = 0;
    return fn() {
        count += 1;
        return count;
    };
}
```

### Error Handling

```ferrite
try {
    let danger = 1 / 0;
    throw "This shouldn't be reached";
} catch err {
    print(f"Caught an error gracefully: {err}");
}
```

### Standard Library (Module System)

Ferrite v1.4.0 automatically embeds a robust standard module system directly into the compiler.

```ferrite
import "mathutils";
import "strings";
import "collections";
import "functional";

print(square(5));
print(pad_left("42", 5, "0"));
print(chunk([1,2,3,4,5], 2));
```

### File I/O

```ferrite
write_file("test.txt", "Hello World");
append_file("test.txt", "!");
if file_exists("test.txt") {
    print(read_file("test.txt"));
}
```

---

## 🛠️ Built-in Functions

Ferrite includes a powerful global standard library available natively:

| Category    | Functions                                                                                                  |
| ----------- | ---------------------------------------------------------------------------------------------------------- |
| **Core**    | `len(x)`, `print(x)`, `write(x)`, `type(x)`, `input(p?)`, `assert(c, m?)`                                 |
| **Math**    | `range(a,b,s)`, `sqrt`, `abs`, `floor`, `ceil`, `round`, `max`, `min`, `pow`, `log`, `sin`, `cos`          |
| **Lists**   | `push`, `pop`, `contains`, `map`, `filter`, `reduce`, `sort`, `reverse`, `enumerate`, `zip`                |
| **Strings** | `str`, `split`, `join`, `replace`, `trim`, `upper`, `lower`, `chars`, `substr`, `starts_with`              |
| **Maps**    | `keys(m)`, `values(m)`, `has_key(m, k)`, `delete(m, k)`                                                    |
| **Files**   | `read_file`, `write_file`, `append_file`, `file_exists`                                                    |

---

## 💡 Implementation Details

- **Modularized Rust** — Zero external dependencies.
- **Hand-written Lexer** — Fast, UTF-8 aware character scanner.
- **Recursive Descent Parser** — Pratt-style precedence for clean expression handling.
- **Semantic Resolver** — Static pass for variable resolution and control flow validation.
- **Bytecode VM** — High-performance stack-based Virtual Machine.
- **Stateful Closures** — Reference-counted capture mechanism for persistent mutable state.
- **Pure Safe Rust** — No `unsafe` code used in the compiler or runtime.
