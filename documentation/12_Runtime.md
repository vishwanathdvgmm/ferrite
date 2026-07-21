# Runtime Architecture

## 1. Overview

Ferrite technically features two distinct runtime backends: an Ahead-Of-Time (AOT) LLVM code generator for native execution, and a pure-Rust, AST-walking **Interpreter**.

Because LLVM IR generation delegates runtime responsibilities directly to the hardware architecture (stack pointers, CPU registers), this document focuses explicitly on the **Interpreter** (`src/runtime/`). The interpreter serves as the execution backend for `ferrite run`, the interactive REPL, and the WebAssembly (WASM) browser playground.

## 2. Runtime Architecture

The Interpreter operates by recursively traversing the Abstract Syntax Tree (AST) that has been structurally verified by the Parser and semantically validated by the Semantic Analyzer.

**Core Components:**

- **`Interpreter` Struct:** The central execution engine. It holds a mutable reference to the global `Environment`.
- **`Value` Enum:** The runtime memory representation of all Ferrite data types (e.g., `Value::Int(i64)`, `Value::Tensor(Vec<f64>, Shape)`).
- **`Environment` Struct:** The runtime symbol table mapped to lexical scopes.

Because the Semantic Analyzer guarantees that type errors, undefined variables, and mismatched shapes do not exist in a successfully checked AST, the Interpreter safely assumes the AST is perfectly sound. It does not perform duplicate type checking at runtime, drastically maximizing evaluation speed.

## 3. Execution Model

The execution model is strictly eager and sequential.
When evaluating a `BinaryExpr` (`a + b`), the interpreter first recursively evaluates the left operand (`a`) to a concrete `Value`, then the right operand (`b`), and finally performs the host-language (Rust) mathematical operation on them, yielding a new `Value`.

### 3.1 Control Flow (Stop and Skip)

Loops (`while`, `for`) execute by recursively evaluating their block bodies.
To support loop modifications like `stop` (break) and `skip` (continue), block evaluation returns a specialized `Result<Value, ControlFlow>` enum rather than a simple `Value`.
When a `stop` statement is evaluated, the interpreter immediately aborts the current block traversal and bubbles the `ControlFlow::Stop` signal up the call stack until it is caught by the nearest loop handler, which terminates execution of the loop and resumes normal block flow.

## 4. Memory Model and Variables

Unlike standard bytecode VMs (which use a linear byte array and an instruction pointer to manage memory), the Ferrite AST Interpreter maps its memory directly to Rust's native heap and stack via the `Environment`.

### 4.1 The `Environment`

An `Environment` is a wrapper around a `HashMap<String, Value>`, storing local variable bindings.
To support lexical scoping, an `Environment` holds an optional `Rc<RefCell<Environment>>` pointing to its enclosing (parent) scope.

### 4.2 Variable Declaration

When a `keep x = 5` statement is executed, the interpreter inserts `("x", Value::Int(5))` into the deepest, currently active `Environment`.

### 4.3 Variable Lookup

When a variable `x` is evaluated, the interpreter looks for it in the current `Environment`. If not found, it traverses the `Rc<RefCell>` chain upwards through parent environments until it reaches the global scope. Because the Semantic Analyzer already proved `x` exists, this lookup is mathematically guaranteed to succeed.

## 5. Function Calls and Stack Frames

Function invocation is the heaviest operation in the interpreter.

### 5.1 Standard Functions

When a function call `add(5, 10)` is evaluated:

1. The arguments are evaluated left-to-right into `Value::Int(5)` and `Value::Int(10)`.
2. The interpreter creates a _new_ `Environment` (a new Stack Frame). Its parent is set to the global environment where the function was originally declared.
3. The evaluated arguments are bound to the function's parameter names (`a = 5`, `b = 10`) inside this new environment.
4. The interpreter recursively evaluates the function's AST body within the context of this new environment.
5. Upon encountering a `return` statement, execution halts, the value bubbles up, and the temporary `Environment` is dropped, cleanly deallocating the local variables.

### 5.2 Closures and Captured State

Lambdas and closures in Ferrite present a memory model challenge. A function returned from another function might reference variables from a scope that has already finished executing.
To support stateful closures safely without a Garbage Collector, Ferrite utilizes Rust's `Rc` (Reference Counted) smart pointers. When a lambda is declared, it takes an `Rc::clone()` of its surrounding `Environment`, ensuring those variables remain alive in memory for exactly as long as the closure itself exists.

## 6. Architectural Trade-offs

### 6.1 Performance Overhead

**Drawback:** An AST-walking interpreter is inherently slow. Every operation requires dynamic dispatch over `Box<Expr>` boundaries, and variable lookups require string hashing (`HashMap<String, Value>`).
**Decision:** This performance overhead is accepted because the interpreter is solely designed for rapid prototyping, REPL interaction, and browser embedding. Production deployment mandates bypassing this module entirely in favor of the `ferrite compile` LLVM backend.

### 6.2 Reference Cycles (Memory Leaks)

**Risk:** Because closures capture scopes via `Rc<RefCell<Environment>>`, it is technically possible to create cyclic data structures (Scope A captures Scope B, Scope B captures Scope A), leading to unrecoverable memory leaks during interpretation.
**Mitigation:** The compiler minimizes this by ensuring `keep` variables are strictly immutable by default, preventing deep cyclical self-referential graph building at runtime unless explicitly forced through `unsafe` patterns.
