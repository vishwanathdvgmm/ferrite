# Ferrite v2.0 — Standard Library

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

| Module         | Description                                      | Status      |
|:---------------|:-------------------------------------------------|:------------|
| `math`         | Numeric utilities: `sqrt`, `abs`, `pow`, `clamp` | 🔲 Planned  |
| `strings`      | String manipulation: `split`, `join`, `trim`     | 🔲 Planned  |
| `collections`  | List utilities: `map`, `filter`, `reduce`, `sort`| 🔲 Planned  |
| `tensor_ops`   | Tensor operations: `matmul`, `reshape`, `zeros`  | 🔲 Planned  |
| `io`           | File and console I/O                             | 🔲 Planned  |

## Using the Legacy Standard Library

If you need the v1.4 standard library, switch to the legacy branch:

```bash
git checkout v1-legacy
cargo build --release
./target/release/ferrite examples.fe
```
