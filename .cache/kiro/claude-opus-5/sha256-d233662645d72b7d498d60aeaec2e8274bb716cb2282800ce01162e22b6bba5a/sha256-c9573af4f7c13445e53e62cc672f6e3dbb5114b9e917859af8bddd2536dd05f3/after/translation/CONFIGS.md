# CONFIGS.md — Phase A configuration-surface table

## How this table was derived (mechanical scan of `c_src/`)

Axes are taken from what the C source actually branches on / special-cases:

```sh
# public entry points (the whole public header, minus licence + include guard):
grep -vE '^//|^$' c_src/include/driver.h
#   #ifndef DRIVER_H_ / #define DRIVER_H_
#   void driver(int x);
#   #endif //DRIVER_H_

# runtime options / modes / flags:
grep -rnE "\b(if|else|switch|case)\b|#if|#ifdef" c_src/src c_src/include
#   -> only the `#ifndef DRIVER_H_` include guard. ZERO runtime branches.

# state / configuration objects:
grep -rnE "\b(enum|struct|typedef|union|static|extern|const)\b" c_src/src c_src/include
#   -> no matches. No globals, no context struct, no init/teardown, no setters.

# cargo feature flags in the Rust crate:
grep -n "\[features\]" translation/Cargo.toml     # -> no matches
```

**Findings that bound this table:**

- The public API is exactly ONE entry point, `void driver(int x)`, and it is
  simultaneously the highest- *and* lowest-level entry point — there is no
  convenience wrapper layered over a lower-level API, no init/config/run
  three-step pipeline, and no state to set up. `nm -D` confirms `driver` is the
  only exported symbol (see `SYMBOLS.md`).
- There are ZERO runtime options, modes or flags: no enum parameters, no
  bitmask flags, no context/handle, no globals, no environment reads, no
  `#ifdef`-selected variants. So the option axis of the cross-product is a
  single point, and the cross-product reduces to the input-shape axis alone.
- The Rust crate declares no `[features]`, so the only feature combination is
  the default one (see the "Feature combinations" section below).

The remaining axes the code genuinely distinguishes are therefore the *value
classes of the single `int` argument*, which fall out of the two arithmetic
operations (`2*x`, `+300`) and of `printf("%d\n", ...)` formatting:

- **arithmetic regime**: no overflow / add-only overflow / multiply overflow
  (positive) / multiply overflow (negative);
- **sign of the printed result**: positive, zero, negative (`%d` emits a `-`);
- **field width of the printed result**: 1..10 digits, i.e. every digit-count
  `%d` can produce, plus the widest negative (`-2147483648`, 11 chars);
- **argument-register shape at the ABI boundary**: clean 32-bit argument vs a
  64-bit argument word with non-zero upper half (callee reads `%edi` only);
- **call multiplicity / stdio state**: single call vs many calls in sequence
  (exercises stdio buffering and that no hidden state accumulates between
  calls), and interleaving C and Rust calls against the same `stdout`.

## Configuration-surface table

Each row = one meaningful combination of the axes above that the C treats
differently. Every row is driven through *both* `.so` exports with MANY
randomized inputs (fixed seed `0x5EED_D1FF_C0FFEE01`, see
`tests/differential.rs`), not one hand-picked value; the assertion is
byte-for-byte equality of everything written to `stdout`.

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | No options exist. Small positive `x` in `[1, 1000]`, no overflow, result positive, 3-4 printed digits. Randomized. | [x] |
| C2 | `driver` | Small negative `x` in `[-1000, -1]`: result may be positive OR negative (`2x+300` crosses zero at `x = -150`), exercising the `-` sign path. Randomized. | [x] |
| C3 | `driver` | `x = 0` — the "empty"/identity input. Prints the bare constant `300`. Deterministic single value. | [x] |
| C4 | `driver` | `x` chosen so the result is exactly `0` (`x = -150`) and exactly `±1` (`x = -149`, `x = -151`) — sign-transition boundary of `%d`. Deterministic. | [x] |
| C5 | `driver` | Result digit-count sweep: `x` chosen so `2x+300` has 1,2,3,...,10 digits, positive and negative, incl. `-2147483648` (widest). Deterministic set + randomized within each width band. | [x] |
| C6 | `driver` | Mid-range `x` in `[1001, 1073741673]`: no overflow at all, large positive result. Randomized. | [x] |
| C7 | `driver` | Add-only-overflow band `x` in `[1073741674, 1073741823]`: `2*x` fits, `+300` wraps. Randomized. | [x] |
| C8 | `driver` | Multiply-overflow band, positive: `x` in `[1073741824, INT_MAX]`. Randomized. | [x] |
| C9 | `driver` | Multiply-overflow band, negative: `x` in `[INT_MIN, -1073741825]`. Randomized. | [x] |
| C10 | `driver` | Mid-range negative, no overflow: `x` in `[-1073741824, -1001]`. Randomized. | [x] |
| C11 | `driver` | Unrestricted full-range `x` drawn uniformly from all 2^32 values (all four arithmetic regimes mixed), many iterations. Randomized. | [x] |
| C12 | `driver` | Powers of two and their neighbours (`±(1<<k)`, `±(1<<k)-1`, `±(1<<k)+1` for `k = 0..31`) — bit-pattern shapes the `lea`/`add` sequence could plausibly special-case. Deterministic sweep. | [x] |
| C13 | `driver` | ABI shape: same value passed through a `void(*)(i64)`-typed pointer with non-zero upper 32 bits (and as an out-of-range "enum" value). Callee must read the low half only. Randomized. | [x] |
| C14 | `driver` | Call multiplicity / no residual state: a long randomized sequence of calls through the SAME `dlopen` handle, output accumulated across all calls and compared as one byte stream (stdio buffering + statelessness). Randomized. | [x] |
| C15 | `driver` | Interleaved C/Rust calls against the same process `stdout` (alternating C, Rust, C, Rust ...), verifying identical interleaving/flush behaviour rather than merely identical per-call bytes. Randomized. | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies, so the complete set of feature combinations is:

| combination | command |
|-------------|---------|
| default (= empty feature set) | `cargo test` |
| explicit no-default-features (identical, since there are no default features) | `cargo test --no-default-features` |

Both are executed by `run_all.sh`; there is no third configuration to cover.

## Are these rows actually discriminating? (mutation evidence)

A green suite is only meaningful if it can fail. Five mutants were injected
into `src/lib.rs`, rebuilt, and run through the same suite; the original file
was then restored and verified byte-identical (`md5sum`
`2483fc6873b78a5d3d5f9e26aa717caf`).

| mutant | change | detected by | rows that correctly still passed |
|--------|--------|-------------|----------------------------------|
| M1 | `wrapping_add(300)` -> `wrapping_add(301)` | 21 of 23 tests, each naming the first diverging input (e.g. `C1: divergence for driver(498)`) | E7 (compares error status, not bytes) and `phase_d_symbol_parity` — correctly insensitive |
| M2 | `wrapping_mul`/`wrapping_add` -> plain `*` / `+`, **debug** profile (overflow checks on) | C7 (panic at the `+300`), C8, C9, E1, E4 (panic at the `2*x`), C11 | C1 — no overflow in that band, so passing is correct |
| M3 | signature widened to `i64`, `(x.wrapping_mul(2) as i32)` | **nobody — and that is correct**: multiplication mod 2^32 is invariant to the upper half, so this mutant is behaviourally equivalent, not a real defect | all rows |
| M4 | high half leaked into the result: `+ ((x >> 32) as i32)` | C13 and E6 only | C1, C11, C12 — they pass clean 32-bit arguments, so passing is correct |
| M5 | libc `printf` -> Rust `println!` (identical bytes, different buffering) | C15 (C's fully-buffered output vs Rust's line-flushed output interleave differently) and E7 (`println!` panics -> `SIGABRT` where the C silently ignores the failed write) | C1, C11, C14 — per-call bytes are identical, which is exactly why the interleaving row exists |

M2 is the evidence that `wrapping_mul`/`wrapping_add` in the translation is
*required*, not incidental: the C compiles to `lea`/`add` on 32-bit registers
and wraps, so any checked-arithmetic translation aborts in a debug build on
inputs the C accepts. M4 and M5 are the evidence that the ABI row (C13) and the
interleaving row (C15) each catch a class of defect no other row catches.
