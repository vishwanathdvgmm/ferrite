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
ferrite/
├─ src/
│  ├─ lexer/     # Token definitions and source scanning
│  ├─ ast/       # Abstract Syntax Tree nodes (Expr, Stmt)
│  ├─ parser/    # Recursive descent & Pratt expression parsing
│  ├─ runtime/   # Tree-walking evaluator, Scopes, and Built-ins
│  ├─ semantic/  # (Future) Static analysis & type checking
│  ├─ codegen/   # (Future) Bytecode emission
│  └─ stdlib/    # Internal registry for linking the external `std/` folder
├─ std/          # The Ferrite Standard Library (written in .fe)
└─ docs/         # Formal language specifications
```

> **Note on `src/stdlib` vs `std/`:**
> The `std/` folder on the root contains the actual *Ferrite code* (like `mathutils.fe`). The `src/stdlib/` folder inside the Rust compiler is a planned module for future updates to register native Rust functions directly into the environment, or to pre-compile/embed the `.fe` files directly into the executable byte-slice so you don't have to distribute the `std/` folder alongside the binary.

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
Ferrite v1.4.0 ships with a robust `std/` module system. Place it next to your binary.

```ferrite
import "std/mathutils";
import "std/strings";
import "std/collections";
import "std/functional";

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

| Category | Functions |
|----------|-----------|
| **Core** | `len(x)`, `print(x)`, `type(x)`, `input(prompt?)` |
| **Math** | `range(a,b,step)`, `sqrt`, `abs`, `floor`, `ceil`, `round`, `max`, `min`, `pow`, `log`, `sin`, `cos` |
| **Lists** | `push`, `pop`, `contains`, `map`, `filter`, `reduce`, `sort`, `reverse`, `enumerate`, `zip` |
| **Strings** | `str`, `split`, `join`, `replace`, `trim`, `upper`, `lower`, `chars`, `substr`, `starts_with`, `ends_with` |
| **Maps** | `keys(m)`, `values(m)`, `has_key(m, k)`, `delete(m, k)` |
| **Files** | `read_file`, `write_file`, `append_file`, `file_exists` |

---

## 🏗️ Project Structure

```
ferrite/
├── Cargo.toml
├── std/
│   ├── mathutils.fe   ← Standard math library
│   ├── strings.fe     ← Extraneous string helpers
│   ├── collections.fe ← List/Map chunking, grouping
│   └── functional.fe  ← Compose, pipe, partials
├── src/
│   └── main.rs        ← entire language in one file
└── examples.fe        ← example programs
```

---

## 💡 Implementation Details

- **~1600 lines** of clean, idiomatic Rust — zero dependencies
- **Lexer** — hand-written character-level scanner
- **Parser** — recursive-descent with Pratt-style precedence climbing
- **Interpreter** — tree-walking with lexical scoping via environment chains
- **Closures** — captured by cloning the environment at definition time (`Rc<RefCell<HashMap>>`)
- **No unsafe** — pure safe Rust throughout
