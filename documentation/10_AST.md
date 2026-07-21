# Abstract Syntax Tree (AST)

## 1. Tree Design

The Abstract Syntax Tree (AST) is the foundational data structure of the Ferrite compiler. It serves as the single source of truth passed from the Front-End (Parser) to the Middle-End (Semantic Analyzer) and Back-End (LLVM/Interpreter).

**Design Philosophy: Strict Data Purity**
The AST is entirely decoupled from execution and validation logic. AST nodes are pure Rust `struct` and `enum` definitions. They do not contain methods for evaluation (`node.eval()`) or type-checking (`node.type_check()`). This deliberate isolation ensures that multiple distinct passes (Import Resolution, Type Checking, LLVM Codegen, AST Interpretation) can traverse the exact same tree without mutating it or entangling their concerns.

## 2. Node Hierarchy

The AST (`src/ast/mod.rs`) is modeled as a tiered hierarchy of algebraic data types (`enums`), accurately reflecting the Ferrite grammar specification.

### 2.1 The Program Root

A Ferrite program is represented as a `Program`, which is simply a `Vec<TopDecl>`.

### 2.2 Top-Level Declarations (`TopDecl`)

The `TopDecl` enum represents constructs that can exist at the root level of a file or module:

- `Function(FunctionDecl)`
- `Group(GroupDecl)`
- `Enum(EnumDecl)`
- `Constant(ConstantDecl)`
- `Import(ImportDecl)`

### 2.3 Statements (`Stmt`)

Statements represent actions that do not produce a value, typically residing inside function bodies or blocks.

- `Keep(KeepStmt)` / `Param(ParamStmt)`: Variable bindings.
- `Expr(ExprStmt)`: An expression executed for side effects.
- `Return(ReturnStmt)`, `Stop`, `Skip`: Control flow modifiers.
- `If(IfStmt)`, `While(WhileStmt)`, `For(ForStmt)`: Block-level control flow.
- `Match(MatchStmt)`: Pattern matching structures.
- `InferBlock(Block)`, `TrainBlock(Block)`: Contextual ML blocks.

### 2.4 Expressions (`Expr`)

Expressions represent constructs that evaluate to a value.

- `Literal(LiteralExpr)`: Primitives (`int`, `float`, `string`, `bool`).
- `Identifier(IdentifierExpr)`: Variable lookups.
- `Binary(BinaryExpr)` / `Unary(UnaryExpr)`: Mathematical and logical operations.
- `Call(CallExpr)`: Function invocations.
- `FieldAccess(FieldAccessExpr)`: `tensor.shape` or `module.func`.
- `GroupLiteral(GroupLiteralExpr)`: Struct initializations.

## 3. Ownership and Memory Model

The AST relies heavily on Rust's `Box<T>` for memory management.

Because an AST is a deeply nested, recursive graph (e.g., a `BinaryExpr` contains left and right `Expr` nodes, which might themselves be `BinaryExpr` nodes), the compiler must allocate these child nodes on the heap to ensure the enum variants have a known, finite size at compile time.

**Immutability:**
Once constructed by the Parser, the AST is strictly **immutable**. The Semantic Analyzer operates via a shared reference (`&AST`) and never mutates the nodes. If a node requires metadata (such as resolved types or symbol IDs), that metadata is stored externally in the `TypeEnv`, keyed by the Node's memory address or a unique Node ID.

## 4. Traversal and The Visitor Pattern

Traversing the AST in Rust typically employs pattern matching (`match node { ... }`).

However, because the AST contains over 34 node variants, manual `match` statements in every compiler pass (Semantic Pass 1, Semantic Pass 2, Interpreter, Codegen) introduce massive boilerplate.

**The Visitor Pattern (Implicit vs Explicit):**
Ferrite currently utilizes explicit, recursive `visit_*` methods on pass-specific struct instances (e.g., the `SemanticAnalyzer` struct has `visit_expr()`, `visit_stmt()`).

Instead of a generic `Visitor` trait that mandates overriding every method (which can obfuscate control flow), Ferrite allows each pass to explicitly match and route the traversal. This makes it trivial to short-circuit traversal (e.g., deciding _not_ to traverse the body of an uncalled generic function during Pass 1).

## 5. Architectural Trade-offs

### 5.1 AST Purity vs TypedAST

**Alternative:** After semantic analysis, many compilers map the raw `AST` to a completely new `TypedAST` struct hierarchy where every node contains a `resolved_type` field.
**Trade-off Accepted:** Ferrite rejects a secondary `TypedAST`. Duplicating the entire AST structure is an expensive, cache-thrashing operation in Rust. Instead, Ferrite side-loads type data into the `TypeEnv` using unique Node IDs. This preserves memory and reduces compilation time, at the cost of requiring the backend to perform Hash Map lookups to retrieve node types.

## 6. Future Changes

- **Node IDs:** Currently, node tracking relies heavily on exact structural traversal paths. Transitioning every AST node to inherently contain a global, monotonically increasing `NodeId` will vastly simplify the `TypeEnv` side-loading architecture.
- **Arena Allocation (Bumpalo):** The extensive use of `Box<T>` causes heap fragmentation. The `ast/mod.rs` module should eventually be refactored to take an arena allocator lifetime (`&'a Bump`), allocating all AST nodes contiguously. This would eliminate recursive `drop` overhead at the end of compilation and maximize CPU cache hit rates during Semantic Analysis.
