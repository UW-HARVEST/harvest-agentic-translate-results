# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

The function is **branch-free** — it has no `if`, no `switch`, no `#ifdef`, and
no runtime options, modes, or flags. So there is no *option* axis:

| axis kind | present? | evidence |
|-----------|----------|----------|
| runtime option / mode / flag | **none** | `lib.h` exposes one function with one scalar parameter and no setters, no context struct, no globals with external linkage (`nm -D` shows only `half2float`) |
| compile-time `#ifdef` | **none** | `grep '#if' c_src/src/lib.c` → 0 matches |
| Cargo features | **none** | `Cargo.toml` has no `[features]` table |

All the meaningful configuration therefore lives in the **input shape**. The
data-dependent branching is implicit, encoded in the lookup tables, and the code
distinguishes these axes:

- **`n = h >> 10`** (0..=63) selects `m__offset[n]` and `m__exponent[n]`.
  - `m__offset[n]` takes exactly **two** distinct values: `0x0000` for
    `n ∈ {0, 32}` and `0x0400` for every other `n`. This selects which **half**
    of `m__mantissa` is used — the two halves have different structure
    (first half: subnormal-encoding values `0x00000000, 0x33800000, …`;
    second half: `0x38000000 + k·0x2000`).
  - `m__exponent[n]` is a regular `n·0x00800000` ramp **except** at
    `n = 31` (`0x47800000`, not `0x0F800000`) and `n = 63` (`0xC7800000`, not
    `0x8F800000`) — the two Inf/NaN special cases. `n ≥ 32` sets the sign bit.
- **`h & 0x3ff`** (0..=1023) is the index within the selected half. Distinguished
  sub-shapes: `== 0` (zero / Inf), `!= 0` (subnormal / NaN payload),
  `== 0x3ff` (upper index bound).
- **`m__mantissa` index bounds**: `0`, `1023` (end of first half), `1024`
  (start of second half), `2047` (end of table).

## Configuration-surface table

Cross-product of {`m__offset` value} × {`m__exponent` regularity class} ×
{sign} × {`h & 0x3ff` sub-shape}, pruned to the combinations the code actually
distinguishes. Every row is exercised with many randomized inputs drawn from
that row's `h`-set (fixed seed, xorshift64* PRNG) plus that row's boundary
values, comparing `half2float` bit patterns from the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | no options (none exist) · `n = 0`, `m__offset = 0`, `h & 0x3ff == 0` → mantissa index 0 · positive zero | [x] |
| 2 | `half2float` | no options · `n = 0`, `m__offset = 0`, `h & 0x3ff != 0` → mantissa index 1..1023 (first table half) · positive subnormals · randomized over all 1023 values | [x] |
| 3 | `half2float` | no options · `n = 0`, `h & 0x3ff == 0x3ff` → mantissa index 1023, upper bound of first half | [x] |
| 4 | `half2float` | no options · `n = 1`, `m__offset = 0x400` → mantissa index 1024, first index of second table half · smallest positive normal exponent | [x] |
| 5 | `half2float` | no options · `n ∈ 2..=30` (regular positive exponent ramp), `m__offset = 0x400`, `h & 0x3ff` randomized · positive normals · randomized over both axes | [x] |
| 6 | `half2float` | no options · `n = 30`, `h & 0x3ff == 0x3ff` → last regular positive normal (largest finite positive half, `h = 0x7BFF`) | [x] |
| 7 | `half2float` | no options · `n = 31` (**irregular** `m__exponent[31] = 0x47800000`), `h & 0x3ff == 0` · `+Inf` | [x] |
| 8 | `half2float` | no options · `n = 31` (irregular exponent), `h & 0x3ff != 0` · positive NaN, payload from second table half · randomized over all 1023 payloads | [x] |
| 9 | `half2float` | no options · `n = 31`, `h & 0x3ff == 0x3ff` → mantissa index 2047 + irregular exponent = arithmetically largest positive sum (`h = 0x7FFF`) | [x] |
| 10 | `half2float` | no options · `n = 32` (**second** `m__offset == 0` entry, sign bit set), `h & 0x3ff == 0` → mantissa index 0 · negative zero | [x] |
| 11 | `half2float` | no options · `n = 32`, `m__offset = 0`, `h & 0x3ff != 0` → first table half with sign bit · negative subnormals · randomized over all 1023 values | [x] |
| 12 | `half2float` | no options · `n = 32`, `h & 0x3ff == 0x3ff` → mantissa index 1023 reached from the negative side | [x] |
| 13 | `half2float` | no options · `n = 33`, `m__offset = 0x400` → mantissa index 1024 from the negative side · smallest negative normal | [x] |
| 14 | `half2float` | no options · `n ∈ 34..=62` (regular negative exponent ramp), `h & 0x3ff` randomized · negative normals · randomized over both axes | [x] |
| 15 | `half2float` | no options · `n = 62`, `h & 0x3ff == 0x3ff` → largest-magnitude finite negative half (`h = 0xFBFF`) | [x] |
| 16 | `half2float` | no options · `n = 63` (**irregular** `m__exponent[63] = 0xC7800000`), `h & 0x3ff == 0` · `-Inf` | [x] |
| 17 | `half2float` | no options · `n = 63` (irregular exponent), `h & 0x3ff != 0` · negative NaN, payload preserved · randomized over all 1023 payloads | [x] |
| 18 | `half2float` | no options · `n = 63`, `h & 0x3ff == 0x3ff` → mantissa index 2047 + irregular negative exponent = arithmetically largest sum overall (`h = 0xFFFF`, bits `0xFFFFE000`) | [x] |
| 19 | `half2float` | no options · every `n` (0..=63) crossed with `h & 0x3ff ∈ {0, 1, 0x1FF, 0x3FE, 0x3FF}` — full offset×exponent×index-boundary cross-product, 320 combinations | [x] |
| 20 | `half2float` | no options · uniformly random `h` over the whole `0x0000..=0xFFFF` domain, 200 000 seeded samples (value-dependent-bug sweep, not restricted to one region) | [x] |
| 21 | `half2float` | no options · **exhaustive**: all 65536 possible inputs, in ascending order, bit-for-bit | [x] |
| 22 | `half2float` | no options · **exhaustive in seeded-shuffled order**, interleaving C and Rust calls — detects any order-dependent/lazily-initialised internal state a translation might have introduced | [x] |
| 23 | `half2float` | no options · concurrent invocation: 8 threads × full domain slice through the same loaded `.so` handles — detects non-thread-safe state | [x] |

## Feature combinations

`Cargo.toml` declares no features, so the default (empty) set is the only
combination. `check_features.sh` enumerates them mechanically and runs the full
test suite under every combination, so this remains valid if features are added.

| combination | `cargo check` | full test suite |
|-------------|---------------|-----------------|
| default (no features declared) | [x] | [x] |
| `--no-default-features` | [x] | [x] |
