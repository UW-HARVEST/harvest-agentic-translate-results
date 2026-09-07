# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/include/lib.h` (the complete public header) and
`c_src/src/lib.c`.

## Axes the C actually branches on

### Runtime options / modes / flags

```
$ grep -nE '#if|#ifdef|#ifndef|#else' c_src/src/lib.c   -> (no matches)
$ grep -nE '\bif\b|\bswitch\b|\?'      c_src/src/lib.c   -> (no matches)
```

**There are none.** The C has no options, no flags, no modes, no global state,
no `#ifdef` variants, and no branches whatsoever. `half2float` is a pure,
stateless, straight-line function: three table lookups, one wrapping add, one
type-pun.

### Public entry points (complete set)

`c_src/include/lib.h` declares exactly one function, and it is simultaneously
the highest- and lowest-level entry point — there is no wrapper/implementation
split to test separately:

| entry point | signature |
|-------------|-----------|
| `half2float` | `float half2float(uint16_t h)` |

No binary/driver executable is built (`c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)`), so the "compare binary stdout" gate is
not applicable.

### Input shapes the code distinguishes

The single input `uint16_t h` is decomposed by the code into exactly two fields,
which select the data-dependent path:

- `n = h >> 10` — the 6-bit high field, index into `m__offset[64]` and
  `m__exponent[64]`. **64 distinct values.**
- `h & 0x3ff` — the 10-bit low field, offset into `m__mantissa[2048]`.
  **1024 distinct values.**

The interesting interaction is `m__offset[n]`, which is `0x0000` for exactly
`n == 0` and `n == 32` (the subnormal/zero rows) and `0x0400` for all other
`n`. That splits the mantissa table into the two halves at index 0..1023 and
1024..2047, so the *combination* of `n` and `h & 0x3ff` is what selects a
mantissa entry — this is the cross-product that must be covered.

## Configuration-surface table

The full cross-product of the two axes is `64 × 1024 == 65536`, i.e. the entire
input domain. Rather than sample it, the test suite covers it **exhaustively**,
so every row below is verified over *all* of its members, not a random subset.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | `n == 0` (`offset 0x0000`, positive zero/subnormal row), all 1024 low-field values `h = 0x0000..0x03FF` | [x] |
| 2 | `half2float` | `n == 32` (`offset 0x0000`, negative zero/subnormal row), all 1024 low-field values `h = 0x8000..0x83FF` | [x] |
| 3 | `half2float` | `n` in `1..=30` (positive normals, `offset 0x0400`), all 1024 low-field values each — 30 × 1024 pairs | [x] |
| 4 | `half2float` | `n in 33..=62` (negative normals, `offset 0x0400`), all 1024 low-field values each — 30 × 1024 pairs | [x] |
| 5 | `half2float` | `n == 31` (positive Inf/NaN row, exponent `0x47800000`), all 1024 low-field values `h = 0x7C00..0x7FFF` | [x] |
| 6 | `half2float` | `n == 63` (negative Inf/NaN row, exponent `0xC7800000`), all 1024 low-field values `h = 0xFC00..0xFFFF` | [x] |
| 7 | `half2float` | low field `== 0` (exact powers of two / zero / Inf) across all 64 `n` values | [x] |
| 8 | `half2float` | low field `== 0x3FF` (max mantissa) across all 64 `n` values | [x] |
| 9 | `half2float` | boundary singletons: `0x0000`, `0x0001`, `0x03FF`, `0x0400`, `0x7BFF`, `0x7C00`, `0x7C01`, `0x7FFF`, `0x8000`, `0xFC00`, `0xFFFF` | [x] |
| 10 | `half2float` | wrapping-add path: every `h` where `m__mantissa[i] + m__exponent[n]` carries into/past bit 31 (all `n >= 32`, plus `n == 31`) | [x] |
| 11 | `half2float` | randomized property sweep, fixed seed (deterministic LCG), 200 000 draws over the full domain | [x] |
| 12 | `half2float` | **exhaustive**: all 65536 inputs `0x0000..=0xFFFF`, bitwise-compared | [x] |
| 13 | `half2float` | repeated / interleaved calls (statelessness & purity: same input after other inputs yields same output in both libs) | [x] |
| 14 | `half2float` | NaN payload preservation: all NaN-producing inputs compared by raw `to_bits()`, not `==` | [x] |

Rows 1–11, 13 and 14 are strict subsets or restatements of row 12, and are
tested explicitly anyway so a divergence is reported against the specific
configuration that failed rather than as one opaque exhaustive failure.

## Comparison method

All comparisons are on the **raw 32-bit pattern** (`f32::to_bits`), never on
`==`, so that NaN results and `-0.0` vs `+0.0` are distinguished
byte-for-byte. Both sides are called only through `libloading` symbols
resolved from the two `.so` files.

## Additional axis found during verification: build profile

The crate has no Cargo features, but it *does* have a second real configuration
axis that changes generated code semantics:

| profile | `overflow-checks` | why it matters |
|---------|-------------------|----------------|
| `release` | off (and `panic = "abort"`) | matches C's silent unsigned wrap |
| `debug`   | **on** | a plain `+` instead of `wrapping_add` would **panic** here |

The C computes `m__mantissa[i] + m__exponent[n]` as a `uint32_t` addition that
genuinely overflows for every `n >= 32` (those exponent entries have bit 31
set). A translation using `+` would pass in release and abort in debug. The Rust
correctly uses `wrapping_add`, and both artifacts are verified separately:
`HALF2FLOAT_PROFILE=debug|release` selects which cdylib is `dlopen`ed, and all
65536 inputs pass under **both** with no overflow panic.

## Harness non-vacuity (mutation testing)

To prove the differential harness can actually detect divergence rather than
passing trivially, five mutants were injected into the Rust and each was caught:

| mutant | result |
|--------|--------|
| one mantissa entry `0x37000000` → `0x37000001` | 5 tests FAILED ✅ caught |
| drop the `m__offset[n]` term from the index | 14 tests FAILED ✅ caught |
| `n = h >> 11` instead of `h >> 10` | 16 tests FAILED ✅ caught |
| low-field mask `0x3ff` → `0x1ff` | 17 tests FAILED ✅ caught |
| `wrapping_add` → `wrapping_sub` | 17 tests FAILED ✅ caught |

`src/lib.rs` was restored afterwards and re-verified table-for-table against
`c_src/src/lib.c` (2048/64/64 entries, zero differences).

Two further guards keep specific tests from being vacuous: `signed_zero_bit_exact`
asserts that `+0.0 == -0.0` under float equality (so only a bitwise comparison
is meaningful), and `row14_nan_payloads_bit_exact` asserts `NaN != NaN` and fails
if it never actually observes a NaN result.

## Feature combinations

```
$ grep -A5 '^\[features\]' translation/Cargo.toml   -> (no [features] section)
```

The crate declares **no** Cargo features and no optional dependencies, so there
is exactly one feature combination (the default, which is also
`--no-default-features`). Both are exercised by `run_all.sh`.

## Completeness

- [x] Every row passes, verified over the exhaustive input domain.
- [x] No binary/driver is built, so no stdout comparison is required.
- [x] Only one feature combination exists; it is covered.
