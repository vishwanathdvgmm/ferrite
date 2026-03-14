# Ferrite Semantics

This document describes the operational semantics of the Ferrite language.

## Compiler Architecture (v1.4.0)

Ferrite is compiled using a multi-stage pipeline:

1. **Lexical Analysis (`src/lexer/mod.rs`)**: Scans UTF-8 source code into token streams.
2. **Parsing (`src/parser/mod.rs`)**: A recursive descent + Pratt parser that transforms tokens into an Abstract Syntax Tree (AST).
3. **AST (`src/ast/mod.rs`)**: Strongly typed intermediate nodes representing expressions and statements.
4. **Execution (`src/runtime/mod.rs`)**: A tree-walking evaluator that dynamically executes the AST. _Note: Future versions will compile AST to a Bytecode VM._

## Everything is an Expression

In Ferrite, almost everything returns a value.

- `if`/`else` blocks return the value of their final evaluated statement.
- Blocks `{ ... }` evaluate to the result of their last statement.
- Functions explicitly require the `return` keyword to pass back a value.
- Assignment operations (`=`, `+=`) evaluate to `null`.
- Empty blocks `{}` evaluate to `null`.

## Mutability and Variables

All variables defined with `let` are fully mutable dynamically typed. Reassignment can change the type of the variable entirely.

```ferrite
let x = 42;
x = "Now I am a string";
```

## Lexical Scoping and Closures

Ferrite uses static (lexical) scoping via environment chains. Environments are stored using `Rc<RefCell<HashMap>>`, allowing true mutable closures.
When a function is defined, it captures its surrounding scope. Variables inside the closure can be modified, and the changes are reflected across multiple invocations.

```ferrite
fn make_counter() {
    let count = 0;
    return fn() {
        count += 1;
        return count;
    };
}
let c = make_counter();
c(); # returns 1
c(); # returns 2
```

## Pass-by-Value (Primitives) vs Pass-by-Reference (Collections)

- **Primitives** (`int`, `float`, `bool`, `string`, `null`) are copied when passed into functions or assigned to new variables.
- **Collections** (`list`, `map`) are passed by reference. Mutating a list or map inside a function mutates the original object.

```ferrite
fn modify_list(lst) {
    lst[0] = 99;
}
let my_list = [1, 2, 3];
modify_list(my_list);
# my_list is now [99, 2, 3]
```

## First-Class Functions

Functions in Ferrite are first-class values (Type `Fn`). They can be:

1. Assigned to variables (`let add = fn(x) { ... };`).
2. Passed as arguments to other functions (`list.map(f)`).
3. Returned from other functions.

## Error Handling

Errors in Ferrite gracefully unwind the stack rather than crashing the interpreter. They can be triggered manually via `throw <expr>` or produced by the core interpreter (e.g., division by zero). These errors propagate up the call stack unless caught by a `try / catch` boundary.

## Variadic Evaluation

When defining a variadic function (`fn my_func(a, ...rest)`), the trailing arguments are collected into a native array and passed precisely as the argument `rest`.
Conversely, Ferrite does NOT currently support "spread" syntax when _calling_ a function.

## Module Import

The `import` statement reads, lexes, parses, and evaluates a file's expressions inline, executing them within the scope of the calling environment. `import "module"` automatically resolves relative to the current file, stopping further imports of the same path if already loaded.
