# Ferrite Type System

Ferrite is dynamically and strongly typed. Variables do not have intrinsic types; values do. Types are resolved securely at runtime during the VM execution phase (`src/runtime/vm.rs`), and invalid operations across arbitrary types (like `5 + "apple"`) will throw a cohesive `Error: Invalid operands for +`, rather than implicitly coercing.

## Primitive Types

At its core, all memory and objects are backed by a single Rust `Value` Enum containing these types:

- **Int (`i64`)**: A 64-bit signed integer.
- **Float (`f64`)**: A 64-bit floating point number.
- **String (`String`)**: A standard UTF-8 string heap-allocated in memory.
- **Bool (`bool`)**: `true` or `false`.
- **Null**: Represents the absence of a value. Always evaluates to false in truthy contexts.

## Compound Types

- **List (`Vec<Value>`)**: A dynamically sized array of mixed types `[1, "two", true]`.
- **Map (`HashMap<String, Value>`)**: Dictionary storage allowing associative `key: value` pairs using string keys. `{"name": "Ferrite"}`.

## Functional Type

- **Fn**: Represents either a User-Defined Function (with its captured closure environment, bytecode chunk, and parameter list) or a Native Rust Builtin function (like `len` or `print`). User functions capture state via a shared `Rc<RefCell<HashMap>>` to support persistent closures.

## Type Conversion & Builtins

Ferrite does not silently coerce types (with the exception of truthy evaluations). You must manually cast using the standard built-ins:

- `int(val)` : Cast to integer.
- `float(val)`: Cast to float.
- `str(val)` : Cast to a string representation.
- `type(val)` : Returns the type of the value as a string (`"int"`, `"float"`, `"string"`, `"bool"`, `"list"`, `"map"`, `"fn"`, `"builtin"`, `"null"`).

### Truthiness

When evaluating `if` conditions or `while` loops, the following rules apply for deciding if an expression is `true`:

- Int `0` is `false`. All other ints are `true`.
- Float `0.0` is `false`. All other floats are `true`.
- `null` is `false`.
- Empty strings `""`, lists `[]`, and maps `{}` are `true` (standard Rust object evaluation).
- `true` is `true`, `false` is `false`.

## String Interpolation

During evaluation of an F-String `f"Value: {expr}"`, the expression evaluates normally, and its result is implicitly wrapped by an equivalent `str(expr)` conversion before concatenation into the final string.
