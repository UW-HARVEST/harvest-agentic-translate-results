# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

Grepping the public header + source for option/mode machinery:

| axis candidate | present in C? |
|---|---|
| runtime option / mode / flag setter | **none** — the only public symbol is `next_double`; no setters, no context config, no globals |
| `#ifdef` / `#if` compile-time variants | **none** |
| element type / width selector | **none** — state is fixed `uint64_t[2]` |
| count / length / size parameter | **none** |
| byte-order handling | **none explicit** — but the C uses type-punning `*(double *)&result`, so the mapping u64-bits → `double` is host-endian-dependent, and the Rust `f64::from_bits` must agree on the host. Exercised by every row (all rows compare raw bit patterns). |

So the configuration surface is **not** flags; it is the *input shape* of the
only input that exists — the 128-bit generator state — plus the *call
sequencing* (the function is stateful, so N-th call ≠ 1st call).

The full set of public entry points is the single lowest-level entry point
`next_double`; there are no convenience wrappers to prefer over it, so every
row drives `next_double` directly through the `.so` export.

Axes actually distinguished by the code:

- **A. `state[0]` (`x`) bit shape** — feeds `x ^= x<<23`, `x ^= x>>17`. Values
  that make the high 23 bits / low 17 bits significant behave differently.
- **B. `state[1]` (`y`) bit shape** — feeds `x ^= y ^ (y>>26)` and the
  `x + y` return, which is the only place unsigned **wrapping overflow** can
  occur (`x.wrapping_add(y)` vs C's modular `uint64_t` add).
- **C. mantissa extraction** — `value >> 12` keeps the top 52 bits; the low 12
  bits of `value` are *discarded*. Rows must include values differing only in
  the low 12 bits (same output) and values differing in bit 12 (different).
- **D. `1023 << 52 | mantissa` then `- 1.0`** — the subtraction is exact for
  results in `[1.0, 2.0)`, and produces `0.0` exactly when mantissa is 0.
- **E. call sequencing / state advance** — 1 call vs 2 vs many; the state must
  match after each call, not just the returned double.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `next_double` | **Degenerate all-zero state**: `state = [0, 0]`. Absorbing: `x` stays 0, return 0, mantissa 0, result exactly `1.0 - 1.0 = 0.0`, state stays `[0,0]`. Repeated 8×. | [x] |
| C2 | `next_double` | **All-ones state**: `state = [u64::MAX, u64::MAX]`. Maximises every shift and forces `x + y` to wrap. | [x] |
| C3 | `next_double` | **`x = 0`, `y` random non-zero** — the `x ^= x<<23; x ^= x>>17` chain is a no-op, only the `y` term contributes. 256 seeds. | [x] |
| C4 | `next_double` | **`x` random non-zero, `y = 0`** — the `y ^ (y>>26)` term and the `+ y` term vanish; pure xorshift on `x`. 256 seeds. | [x] |
| C5 | `next_double` | **Both random full-range** `u64` (the general case). 4096 seeds, single call each. | [x] |
| C6 | `next_double` | **Wrapping-add boundary (axis B)**: `y` chosen so the post-shift `x + y` crosses `2^64` (`x` random, `y = 0u64.wrapping_sub(x_after)` ± small delta), plus `x+y == 2^64-1` and `== 2^64` exactly. | [x] |
| C7 | `next_double` | **Low-12-bits-discarded equivalence (axis C)**: pairs of states whose `cn_rnd_next` results differ only in bits 0..11 must give the *same* double; differing in bit 12 must give a *different* one. Verified against C, not assumed. | [x] |
| C8 | `next_double` | **Mantissa extremes (axis D)**: states driven to mantissa `0` (result `0.0`), mantissa `0xF_FFFF_FFFF_FFFF` (result nextafter(1.0,0) below 1.0), and mantissa `1` (smallest positive `2^-52`). Exact bit compare of the returned `double`. | [x] |
| C9 | `next_double` | **Single-bit states (axis A×B sweep)**: for every bit position i in 0..64, the three states `(1<<i, 0)`, `(0, 1<<i)` and `(1<<i, 1<<i)` — 192 states covering every bit in isolation — catches off-by-one in the 23/17/26 shift constants. | [x] |
| C10 | `next_double` | **Alternating / structured bit patterns**: `0xAAAA...`, `0x5555...`, `0xFFFF_FFFF_0000_0000`, `0x0000_0000_FFFF_FFFF`, `0x8000...0`, `0x1`, and their 6×6 cross-product for `(x, y)`. | [x] |
| C11 | `next_double` | **Long sequential run (axis E)**: one shared state, 100 000 consecutive `next_double` calls; every returned `double` bit pattern AND the full 128-bit state after every call compared C-vs-Rust. This is the "full operation end to end" pipeline case. | [x] |
| C12 | `next_double` | **Many independent short runs (axis E)**: 1024 random seeds × 16 calls each, comparing the whole 16-value output vector and the final state — catches divergence that only shows up after state feedback. | [x] |
| C13 | `next_double` | **Range invariant across a large sample**: every result from a 200 000-sample run must lie in `[0.0, 1.0)` for BOTH libraries and be bit-identical (guards against a Rust `from_bits` / endianness mismatch producing an in-range-looking but different value). | [x] |
| C14 | `next_double` | **In/out state aliasing & struct layout**: caller passes the same `cn_rnd_t` repeatedly and also reads `state` between calls; asserts `size_of::<cn_rnd_t>() == 16`, `align == 8`, and that Rust writes back both words in the same order/positions as C. | [x] |

## Completion

- [x] Every row passes across randomized inputs (fixed seed, reproducible).

Randomization uses a fixed-seed SplitMix64 defined in the test file, so runs are
byte-for-byte reproducible and independent of any external RNG crate.

## How to reproduce

```bash
# C reference .so
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# Rust cdylib + differential suite, all feature combos x both profiles
cd translation && bash tools/check_features.sh
```

`tools/check_features.sh` extracts the feature list from `Cargo.toml`
(currently empty ⇒ 1 combination), builds the cdylib in the matching profile
(`cargo test` does not build a cdylib-only lib target), and runs all 23 tests
under both `debug` and `release`.

## Harness self-check (mutation test)

To confirm the rows are not vacuously passing, the shift constant in the Rust
`cn_rnd_next` was flipped from `>> 17` to `>> 18` and the suite re-run:
**14 of the 15 Phase B tests failed** in both profiles (only C1, the all-zero
absorbing state, is insensitive to that constant — as expected, since every
intermediate is 0). The constant was then restored and the suite passes again.
