# Testing Architecture

## 1. Testing Philosophy

Compiler verification requires absolute rigidity. A single regression in the Semantic Analyzer can silently compile flawed neural network logic into a production binary. To prevent this, Ferrite employs a **Black-Box, End-to-End (E2E) Verification Strategy**.

While unit testing individual internal Rust modules (e.g., asserting that the lexer produces exactly 5 tokens for a given string) is useful for early development, it results in highly brittle tests that break every time the AST structure changes.

Instead, the Ferrite test suite treats the compiler as a black box. It feeds `.fe` source files into the CLI and asserts that the compiler produces the exact expected terminal output (either a successful execution result or a specific ANSI-formatted compilation error).

## 2. Test Structure

The testing framework is located entirely within the `tests/` directory and is orchestrated by a master shell script (`tests/run_tests.sh`).

### 2.1 Pass Tests (`pass_*.fe`)

Pass tests contain valid Ferrite code. The verification script compiles and runs these files.
**Success Criteria:** The script must exit with a `0` status code, and any `print()` output must exactly match the expected stdout pattern defined for that test.
**Coverage Areas:** Primitives, Generics, ML Blocks, Closures, Loops, Exhaustive Matching, and Module Imports.

### 2.2 Fail Tests (`fail_*.fe`)

Fail tests contain intentionally malformed code (syntax errors, type mismatches, dimensionality clashes).
**Success Criteria:** The compiler must _fail_ (exit code non-zero), and more importantly, it must emit the exact expected error string from the `DiagnosticBag` without panicking the Rust thread.
**Coverage Areas:** Type coercion attempts, undefined variables, missing semicolons, private module access violations, and trait bound violations.

### 2.3 Runtime Fail Tests (`runtime_fail_*.fe`)

These tests contain semantically valid code that is logically flawed (e.g., division by zero, array index out of bounds).
**Success Criteria:** The AST must pass semantic validation, but execution must safely abort with a deterministic runtime trap, rather than triggering undefined behavior or a host OS segfault.

## 3. Coverage Strategy

The test suite is structured to independently verify every layer of the compilation pipeline:

1. **Lexical Integrity:** Tests intentionally injecting invalid unicode or unclosed strings.
2. **Syntactic Recovery:** Tests with missing braces (`fail_09_syntax_missing_brace.fe`) verify that Panic-Mode error recovery successfully synchronizes and reports the error without crashing.
3. **Semantic Rigidity:** The core of the test suite. Tests verify that the Unification Engine correctly infers complex generic shapes and explicitly rejects invalid types (`fail_01_type_mismatch.fe`).
4. **Execution Soundness:** Tests involving deep recursion (`pass_21_deep_recursion.fe`) and nested stateful closures (`pass_20_nested_closures.fe`) verify that the execution environment correctly handles stack frames and reference counting without leaking memory or corrupting state.

## 4. Regression Strategy

Ferrite employs a zero-tolerance regression policy.

1. **Automated Verification:** The `run_tests.sh` script is integrated into the CI/CD pipeline. No commit can be merged to the `main` branch unless all tests pass.
2. **Bug-Driven Test Creation:** When a bug is reported (e.g., Rust-Analyzer desynchronizing on `write!` macros in v2.4.1), a minimal reproducible `.fe` file must be created and added to the test suite _before_ the compiler logic is patched. This ensures the bug can never silently reappear in future versions.

## 5. Edge Cases & Known Testing Limitations

### 5.1 LLVM Codegen Testing

**Limitation:** Currently, `run_tests.sh` relies heavily on the `ferrite run` (Tree-Walk Interpreter) command to verify execution logic, due to the complexity of building LLVM across diverse CI environments.
**Future Requirement:** The test suite must be expanded to run all `pass_*.fe` tests twice: once via the Interpreter, and once via the LLVM AOT backend, diffing the stdout of both runs to guarantee that semantic lowering is identical across backends.

### 5.2 Stdlib Mocking

**Limitation:** Tests evaluating file I/O (`read_file`, `write_file`) currently write real files to the disk during execution.
**Future Requirement:** The runtime needs an abstract `VirtualFileSystem` trait to mock OS interactions during testing, preventing tests from leaving zombie files in the `/tmp` directory upon failure.
