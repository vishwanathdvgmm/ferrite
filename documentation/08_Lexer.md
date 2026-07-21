# Lexer Architecture

## 1. Theory

The Lexer (or Tokenizer) is the first phase of the Ferrite compiler pipeline. Its sole mathematical responsibility is to perform lexical analysis: converting an unstructured, raw stream of UTF-8 characters (`&str`) into a structured, sequential stream of non-overlapping `Token` objects.

By grouping raw characters into meaningful atomic units (like `Identifier`, `IntLiteral`, or `KeywordIf`), the Lexer drastically simplifies the complexity of the subsequent recursive-descent Parser.

## 2. Architecture

The Ferrite Lexer (`src/lexer/mod.rs`) is implemented as a deterministic, stateful, single-pass character scanner. It does not use Regular Expressions (Regex) for matching.

**Core Components:**

- `Lexer`: The main struct holding a reference to the source string (`source: &str`), the current byte position (`pos: usize`), and tracking line/column offsets for error reporting.
- `Token`: A lightweight struct containing the `TokenKind` (enum) and a `Span`.
- `Span`: A precise metadata struct (`start`, `end`, `line`, `column`) pointing back to the exact physical location of the token in the source string.
- `TokenKind`: An algebraic data type (`enum`) defining the 34 reserved keywords, operators, and literals.

## 3. Token Definitions

Tokens in Ferrite are strictly categorized to provide maximum semantic context to the parser without crossing into syntactical evaluation.

### Categories

1. **Single-Character Operators:** `+`, `-`, `*`, `/`, `%`, `{`, `}`, `[`, `]`, `(`, `)`, `:`, `;`, `,`, `.`, `=`
2. **Multi-Character Operators:** `==`, `!=`, `<=`, `>=`, `=>`, `->`, `&&`, `||`
3. **Keywords:** Explicitly reserved words (e.g., `fun`, `if`, `train`, `Tensor`). See `07_Language_Design.md`.
4. **Literals:** `IntLiteral(i64)`, `FloatLiteral(f64)`, `StringLiteral(String)`, `BoolLiteral(bool)`.
5. **Identifiers:** User-defined names matching the regex `[a-zA-Z_][a-zA-Z0-9_]*`.
6. **EOF:** A special End-Of-File marker signaling the termination of the token stream.

## 4. Algorithms & Tokenization Process

The tokenization process relies on a tight `while` loop over a `Peekable<Chars>` iterator (or manual byte indexing for extreme performance).

### The Core Loop Algorithm

1. **Skip Whitespace & Comments:** The lexer advances `pos` past spaces, tabs, newlines (updating the `line` counter), and single-line comments starting with `//`.
2. **Match Next Byte:** The lexer inspects the character at `pos`.
3. **Dispatch:**
   - If the character is alphabetical or `_`: Delegate to `scan_identifier_or_keyword()`.
   - If the character is a digit: Delegate to `scan_number()`.
   - If the character is `"`: Delegate to `scan_string()`.
   - Otherwise, perform a `match` on operators.
4. **Consume & Emit:** Advance `pos` by the length of the matched sequence, instantiate a `Token`, and push it to the output vector.

### 5. Edge Cases

- **Float vs Integer Resolution:** When `scan_number()` encounters digits, it assumes an `IntLiteral`. If it encounters a single `.` followed immediately by more digits, it pivots state to parse a `FloatLiteral`. If the `.` is not followed by digits, it emits the integer and leaves the `.` to be parsed as a field-access operator (e.g., `tensor.shape`).
- **Multi-Character Operator Ambiguity:** When the lexer sees `=`, it must `peek()` the next character. If it is `=`, it consumes both and emits `==`. Otherwise, it emits `=`.
- **String Escaping:** `scan_string()` must correctly handle escaped characters (e.g., `\n`, `\"`) without terminating the string literal prematurely.

## 6. Performance

The Lexer is currently the fastest phase in the compiler pipeline.

### Architectural Decisions for Speed

- **Zero-Copy Identifiers (Planned):** Currently, identifiers and string literals might allocate `String` objects on the heap. A critical optimization is to use `&'a str` slices pointing directly into the original source buffer, avoiding allocations completely.
- **Avoid Regex:** Regex engines introduce massive finite-state machine overhead. Hand-rolled character switching (`match c { ... }`) compiles down to highly optimized jump tables in LLVM/Rust.

### Complexity

- **Time Complexity:** $O(N)$ where $N$ is the number of bytes in the source code. Every byte is inspected at most twice (once for `peek`, once for `consume`).
- **Space Complexity:** $O(T)$ where $T$ is the number of resulting tokens in the file.

## 7. Future Improvements

- **String Interning:** Replace heap-allocated `String` identifiers in `TokenKind::Identifier(String)` with an interned `Symbol` (a 32-bit integer mapping to a global string registry). This dramatically shrinks the `Token` size in memory and converts string equality checks (used heavily in the Semantic Analyzer) into lightning-fast integer comparisons.
- **Multi-line Comments:** Add support for `/* ... */` block comments, properly handling nested comment edge cases.
- **Parallel Lexing:** For massively parallel compilation, the source code could be split into chunks (broken on newlines) and lexed concurrently across CPU threads. However, single-threaded lexing is currently so fast that parallel overhead might outweigh the benefits.
