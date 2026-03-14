# 🦀 Ferrite

A clean, expressive scripting language — written entirely in Rust.

---

## Quick Start

```bash
# Build
cargo build --release

# Run a file
./target/release/ferrite examples.fe

# Start the REPL
./target/release/ferrite
```

---

## Language Tour

### Variables
```ferrite
let x = 42;
let name = "Ferrite";
let pi = 3.14159;
let flag = true;
let nothing = null;
```

### Arithmetic & Strings
```ferrite
let a = 10 + 3 * 2;     // 16  (standard precedence)
let b = 10 / 3;          // 3.3333...
let c = 10 % 3;          // 1
let s = "Hello, " + name;
let rep = "ha" * 3;      // "hahaha"
```

### Lists
```ferrite
let nums = [1, 2, 3, 4, 5];
print(nums[0]);       // 1
print(nums[-1]);      // 5  (negative indexing)
nums[2] = 99;         // mutation

// Concatenation
let more = nums + [6, 7, 8];
```

### Control Flow
```ferrite
if score >= 90 {
    print("A");
} else {
    print("B or lower");
}
```

### Loops
```ferrite
// while
let i = 0;
while i < 10 {
    print(i);
    i = i + 1;
}

// for over a range
for n in range(5) { print(n); }        // 0..4
for n in range(1, 11) { print(n); }    // 1..10
for n in range(0, 10, 2) { print(n); } // 0, 2, 4, 6, 8

// for over a list
for item in ["a", "b", "c"] { print(item); }

// for over a string (character by character)
for ch in "hello" { print(ch); }
```

### Functions
```ferrite
fn add(a, b) {
    return a + b;
}
print(add(3, 4));   // 7
```

### Closures & First-Class Functions
```ferrite
fn make_counter() {
    let count = 0;
    return fn() {
        count = count + 1;
        return count;
    };
}

let c = make_counter();
print(c());  // 1
print(c());  // 2
```

### Anonymous Functions (Lambdas)
```ferrite
let square = fn(x) { return x * x; };
print(square(5));  // 25
```

### String Properties
```ferrite
let s = "  Hello World  ";
print(s.upper);   // "  HELLO WORLD  "
print(s.lower);   // "  hello world  "
print(s.trim);    // "Hello World"
print(s.len);     // 15
print(s.chars);   // list of characters
```

---

## Built-in Functions

| Function           | Description                          |
|--------------------|--------------------------------------|
| `len(x)`           | Length of list or string             |
| `push(list, val)`  | Returns new list with val appended   |
| `pop(list)`        | Returns last element                 |
| `str(x)`           | Convert to string                    |
| `int(x)`           | Convert to integer                   |
| `float(x)`         | Convert to float                     |
| `type(x)`          | Returns type name as string          |
| `range(n)`         | List `[0..n)`                        |
| `range(a, b)`      | List `[a..b)`                        |
| `range(a, b, step)`| List with step                       |
| `sqrt(n)`          | Square root                          |
| `abs(n)`           | Absolute value                       |
| `floor(n)`         | Floor to integer                     |
| `ceil(n)`          | Ceiling to integer                   |
| `max(a, b, ...)`   | Maximum of values                    |
| `min(a, b, ...)`   | Minimum of values                    |
| `input(prompt?)`   | Read a line from stdin               |
| `print(x)`         | Print a value                        |

---

## Operators

| Category   | Operators                  |
|------------|----------------------------|
| Arithmetic | `+  -  *  /  %`            |
| Comparison | `==  !=  <  <=  >  >=`     |
| Logic      | `&&  \|\|  !`              |
| Assignment | `=`                        |
| String     | `+` (concat), `*` (repeat) |
| List       | `+` (concat)               |

---

## Project Structure

```
ferrite/
├── Cargo.toml
├── src/
│   └── main.rs        ← entire language in one file
│       ├── Lexer      (tokenisation)
│       ├── Parser     (recursive-descent → AST)
│       ├── Interp     (tree-walking interpreter)
│       └── main()     (REPL + file runner)
└── examples.fe        ← example programs
```

---

## Implementation Details

- **~700 lines** of clean, idiomatic Rust — zero dependencies
- **Lexer** — hand-written character-level scanner
- **Parser** — recursive-descent with Pratt-style precedence climbing
- **Interpreter** — tree-walking with lexical scoping via environment chains
- **Closures** — captured by cloning the environment at definition time
- **No unsafe** — pure safe Rust throughout
