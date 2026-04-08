# Ferrite v2.1 — Standard Library

> **Status: Implemented (v2.1.0)**
>
> The Ferrite v2.1 standard library has been successfully migrated to the new statically-typed AOT compiler. All modules are embedded at compile time and resolved through a refined import system.

## Implementation Details

The standard library consists of Ferrite source files embedded into the compiler binary. They utilize recursive generics and the refined type system to provide zero-cost abstractions for common tasks.

1. **Embedded Assets**: Modules are bundled via `include_str!` in `src/stdlib/mod.rs`.
2. **Static Resolution**: `import "name"` calls are intercepted by the `ImportResolver` to load these embedded scripts.
3. **Type Safety**: All functions are fully typed and pass the unified semantic analyzer.

## Available Modules

| Module        | Description                                       | Status      |
| :------------ | :------------------------------------------------ | :---------- |
| `math`        | Numeric utilities: `sin`, `cos`, `pow`, constants | ✅ Finished |
| `strings`     | String manipulation: `split`, `upper`, `trim`     | ✅ Finished |
| `collections` | Generic `List<T>` and `Map<K, V>` utilities       | ✅ Finished |
| `io`          | File and console I/O                              | ✅ Finished |

## Usage

Standard modules can be imported directly. No external library files are needed.

```ferrite
import "math";
import "strings";

fun main() {
    keep val: float = math.sin(math.PI);
    keep msg: string = strings.lower("HELLO");
}
```

## Legacy Access

The v1.4.0 standard library remains available on the [`v1-legacy`](https://github.com/vishwanathdvgmm/ferrite/tree/v1-legacy) branch for projects using the older Bytecode VM.

> **Status: Placeholder**
>
> The Ferrite v2.0 standard library has not yet been migrated to the new statically-typed AOT compiler. The v1.4.0 standard library modules (`mathutils`, `strings`, `collections`, `functional`) remain available on the [`v1-legacy`](https://github.com/vishwanathdvgmm/ferrite/tree/v1-legacy) branch.

## What Changed

In v1.4.0, the standard library consisted of `.fe` scripts embedded at compile time via `include_str!` and executed by the bytecode VM. This approach is incompatible with v2.0's ahead-of-time compilation model.

The v2.0 stdlib will need to:

1. Be written in Ferrite v2.0 syntax (typed `fun` declarations with `keep`/`param`)
2. Pass the static type checker
3. Be compiled to native code alongside user programs

## Planned Modules

| Module        | Description                                       | Status     |
| :------------ | :------------------------------------------------ | :--------- |
| `math`        | Numeric utilities: `sqrt`, `abs`, `pow`, `clamp`  | 🔲 Planned |
| `strings`     | String manipulation: `split`, `join`, `trim`      | 🔲 Planned |
| `collections` | List utilities: `map`, `filter`, `reduce`, `sort` | 🔲 Planned |
| `tensor_ops`  | Tensor operations: `matmul`, `reshape`, `zeros`   | 🔲 Planned |
| `io`          | File and console I/O                              | 🔲 Planned |

## Using the Legacy Standard Library

If you need the v1.4 standard library, switch to the legacy branch:

```bash
git checkout v1-legacy
cargo build --release
./target/release/ferrite examples.fe
```
