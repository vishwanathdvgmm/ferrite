{{Infobox programming language
| name                   = Ferrite
| logo                   = [[File:Ferrite Logo.png|120px]]
| paradigm               = Multi-paradigm: [[functional programming|functional]], [[imperative programming|imperative]], [[concurrent programming|concurrent]], [[array programming|array / tensor-oriented]]
| designed_by            = Vishwanath M M
| developer              = Vishwanath M M and Ferrite Contributors
| released               = {{Start date and age|2026|01|15}}
| latest_release_version = 3.2.1
| latest_release_date = {{Start date and age|2026|09|02}}
| typing = [[Static typing|Static]], [[Strong and weak typing|strong]], [[Nominal type system|nominal]], [[Type inference|inferred]]
| scope = [[Lexical scope|Lexical]]
| programming_language = [[Rust (programming language)|Rust]]
| platform = [[x86-64]], [[ARM architecture|ARM64]]
| os = [[Microsoft Windows]], [[macOS]], [[Linux]]
| license = [[MIT License|MIT]] / [[Apache License|Apache-2.0]]
| file_ext = .fe
| website = {{URL|https://www.ferrite-lang.org/}}
}}

'''Ferrite''' is a [[Statically typed programming language|statically-typed]], [[Ahead-of-time compilation|ahead-of-time compiled]], and [[tree-walk interpreter|interpreted]] [[programming language]] developed specifically for [[artificial intelligence]], [[machine learning]], and [[systems programming]]. Designed and created by Vishwanath M M in 2026, the language implementation is written in [[Rust (programming language)|Rust]].

Ferrite features native multi-dimensional [[tensor]] primitives with compile-time shape verification, zero implicit type coercion, compiler-generated [[automatic differentiation]], native dynamic data structures, and scope-delimited execution contexts (such as <code>train</code> and <code>infer</code> blocks). The language natively supports the matrix multiplication operator <code>@</code> and tensor initializers like <code>rand</code>, <code>ones</code>, and <code>zeros</code>.

== History ==
Development of Ferrite began in early 2026 to address productivity and runtime safety challenges in machine learning software development. While languages such as [[Python (programming language)|Python]] dominate machine learning research due to dynamic flexibility, they frequently suffer from runtime shape mismatch errors, heavy interpreter overhead, and dynamic typing bugs. Conversely, native systems languages such as [[C++]] or [[Rust (programming language)|Rust]] require complex foreign function interface (FFI) bindings to interface with high-level tensor computing libraries.

Ferrite was designed as an "AI-Native" compiled language that integrates tensor algebra directly into the language specification and type checker.

- '''v1.0.0''' (January 2026): Initial release featuring the core lexer, recursive descent parser, and tree-walk interpreter supporting primitive types, basic control flow, and unshaped arrays.
- '''v1.4.0''' (March 2026): Introduced nominal struct types (<code>group</code>), interface contracts (<code>trait</code>), and algebraic data types (<code>enum</code>) with pattern matching.
- '''v2.0.0''' (May 2026): Integrated compile-time shaped tensor generics (<code>Tensor<T, Shape></code>) and matrix multiplication operators.
- '''v2.4.0''' (July 2026): Added a full module import/export system with symbol visibility controls (<code>pub</code>) and explicit symbol aliasing.
- '''v2.4.1''' (July 2026): Added first-class function closures and published the 21-chapter Engineering Knowledge Base (EKB) compiler specification.
- '''v2.0.0''' (May 2026): Integrated compile-time shaped tensor generics (<code>Tensor<T, Shape></code>) and matrix multiplication operators.
- '''v2.4.0''' (July 2026): Added a full module import/export system with symbol visibility controls (<code>pub</code>) and explicit symbol aliasing.
- '''v2.4.1''' (July 2026): Added first-class function closures and published the 21-chapter Engineering Knowledge Base (EKB) compiler specification.
- '''v3.0.0''' (August 2026): Released official VS Code extension and native LLVM operator generation.
- '''v3.1.0''' (August 2026): Shifted to an expression-oriented architecture and scope-based memory management.
- '''v3.2.0''' (September 2026): Introduced native dynamic collections (Lists, Maps), string methods, advanced iteration semantics, and comprehensive Machine Learning primitives including native multidimensional Tensors, the <code>@</code> operator, execution blocks, and tensor built-ins.

== Design and philosophy ==

=== Strict type safety and zero coercion ===
Ferrite enforces absolute type safety. The compiler does not perform implicit type coercion (such as automatically promoting an integer to a floating-point number or evaluating non-boolean values in conditional statements). Numeric conversions require explicit cast invocation (e.g., <code>float(x)</code>).

=== Compile-time tensor shape verification ===
Tensors in Ferrite are first-class language primitives. Tensor dimensions are declared within generic type signatures (e.g., <code>Tensor<float, (100, 784)></code>). During semantic analysis, the compiler validates matrix operations (such as matrix multiplication via the <code>@</code> operator) against dimension compatibility rules at compile-time, eliminating runtime dimension mismatch errors.

=== Execution context blocks ===
Ferrite provides dedicated language blocks to declare execution intent:

- <code>train { ... }</code>: Enables automatic differentiation tracking and gradient calculation buffers.
- <code>infer { ... }</code>: Disables gradient tracking and backpropagation overhead, optimizing execution purely for forward-pass inference.

=== Dual execution model ===
The reference Ferrite toolchain includes both an interactive [[tree-walk interpreter]] for rapid prototyping and an [[LLVM]] [[ahead-of-time compilation|ahead-of-time (AOT)]] compiler backend for producing standalone native executables.

== Syntax and semantics ==

=== Variables and mutability ===
Variables declared with the <code>keep</code> keyword are [[immutable object|immutable]] by default. Reassignment requires explicit variable mutability.

<syntaxhighlight lang="rust">
// Immutable declaration
keep pi: float = 3.14159;

// Reassignable variable
keep count: int = 0;
count = count + 1;
</syntaxhighlight>

=== Functions and closures ===
Functions are declared using the <code>fun</code> keyword with explicit parameter types and return type annotations. Ferrite supports first-class closures that capture variables from their outer lexical scope.

<syntaxhighlight lang="rust">
fun add(a: int, b: int) -> int {
    return a + b;
}

// Anonymous closure capturing outer variable
keep factor: int = 2;
fun double(x: int) -> int {
return x \* factor;
}
</syntaxhighlight>

=== Pattern matching ===
Ferrite features algebraic pattern matching on enums with support for conditional guard clauses.

<syntaxhighlight lang="rust">
enum Result<T, E> {
    Ok(T);
    Err(E);
}

keep status: Result<int, string> = Ok(200);

match status {
case Ok(code) if code == 200 => {
println("Success");
}
case Ok(code) => {
println("Status: " + str(code));
}
case Err(msg) => {
println("Error: " + msg);
}
}
</syntaxhighlight>

=== Groups and traits ===
Ferrite uses <code>group</code> for aggregate struct definitions and <code>trait</code> for defining abstract interfaces.

<syntaxhighlight lang="rust">
group Point {
    x: float;
    y: float;
}

trait Summary {
fun summarize(self) -> string;
}

impl Summary for Point {
fun summarize(self) -> string {
return "Point(" + str(self.x) + ", " + str(self.y) + ")";
}
}
</syntaxhighlight>

== Examples ==

=== Tensor matrix multiplication ===
<syntaxhighlight lang="rust">
import "math";

// Validated matrix dimensions: (100, 784) x (784, 10) => (100, 10)
param inputs: Tensor<float, (100, 784)> = ones();
param weights: Tensor<float, (784, 10)> = rand();

infer {
keep logits: Tensor<float, (100, 10)> = inputs @ weights;
println("Output tensor computed successfully.");
}
</syntaxhighlight>

== Implementation ==
The reference implementation of the Ferrite compiler is written in [[Rust (programming language)|Rust]]. Its internal architecture comprises:

# '''Lexer''': Performs lexical analysis, converting source code into a stream of typed tokens.

# '''Parser''': Recursive descent parser that constructs an [[Abstract Syntax Tree]] (AST).

# '''Semantic Analyzer''': Performs symbol table resolution, nominal type checking, and tensor dimension shape unification.

# '''Interpreter & Codegen''': Evaluates AST nodes via a Tree-Walk runtime or emits [[LLVM]] [[Intermediate representation|IR]] for native binary compilation.

== See also ==

- [[Rust (programming language)]]
- [[Python (programming language)]]
- [[Julia (programming language)]]
- [[Mojo (programming language)]]
- [[LLVM]]
- [[Automatic differentiation]]

== References ==
{{reflist|refs=
<ref name="official-site">{{cite web |title=Ferrite Programming Language |url=https://www.ferrite-lang.org/ |access-date=2026-07-23}}</ref>
<ref name="ekb-docs">{{cite web |title=Ferrite Architecture & Engineering Knowledge Base |url=https://github.com/vishwanathdvgmm/ferrite/tree/main/documentation |access-date=2026-07-23}}</ref>
<ref name="repo">{{cite web |title=Ferrite Source Code Repository |url=https://github.com/vishwanathdvgmm/ferrite |publisher=GitHub |access-date=2026-07-23}}</ref>
}}

== External links ==

- {{Official website|https://www.ferrite-lang.org/}}
- [https://github.com/vishwanathdvgmm/ferrite Official GitHub Repository]
- [https://www.ferrite-lang.org/docs/ Ferrite Language Documentation]

[[Category:Programming languages]]
[[Category:Statically typed programming languages]]
[[Category:Compilers]]
[[Category:Rust (programming language) software]]
[[Category:Machine learning software]]
[[Category:Programming languages created in 2026]]
