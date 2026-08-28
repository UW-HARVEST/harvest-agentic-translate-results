# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## Mechanical derivation of the axes

### Public entry points (the FULL set, lowest level included)

```
$ grep -nE '\(' c_src/include/lib.h
1:void hsl_to_rgb(float *dest, const float *src);

$ nm -D --defined-only c_src/build/lib*.so
0000000000001109 T hsl_to_rgb
```

There is exactly **one** exported function. There is no init/teardown pair, no
context object, no convenience wrapper over a lower-level primitive, and no
private helper with external linkage — so "exercise the low-level entry points,
not only the convenience wrappers" collapses to: call `hsl_to_rgb` itself, which
*is* the lowest level. Every row below drives that one entry point.

### Runtime options / modes / flags

```
$ grep -nE 'static|extern|global|set_|_set|struct|typedef|enum|union' c_src/src/lib.c c_src/include/lib.h
(no matches)
$ grep -nE '^\s*#' c_src/src/lib.c c_src/include/lib.h
1:#include <math.h>
3:#include "lib.h"
$ grep -iE 'option|add_definitions|CMAKE_C_FLAGS|BUILD_TYPE' c_src/CMakeLists.txt
(no matches)
$ grep -A5 '\[features\]' translation/Cargo.toml
(no [features] section)
$ grep -c 'feature' translation/src/lib.rs
0
```

**There are no runtime options, no global/static state, no `#ifdef`s, no CMake
options and no cargo features.** The library is a pure stateless function.
Consequently there is exactly **one** feature combination to verify
(`--no-default-features` and the default build are the same build); Phase D's
"repeat for every feature combination" is satisfied by that single combination,
and the driver script still runs the suite under both invocations to prove it.

### The axes the C actually branches on

Everything the code distinguishes is a property of the three input floats plus
the two pointers:

* **A1 — hue sector.** The `if/else if` chain at `lib.c:19-47` selects one of
  **7** outcomes. Note that these are *not* the six 60° sectors a reader expects,
  because `lib.c:27` tests `h < 120.0f && h < 180.0f`:
  | outcome | condition actually required | stores |
  |---|---|---|
  | B1 | `0 <= h < 60`   | `{c+m, x+m, m}` |
  | B2 | `60 <= h < 120` | `{x+m, c+m, m}` |
  | B3 | `h < 0` (only way to reach line 27) | `{m, c+m, x+m}` |
  | B4 | `180 <= h < 240`| `{m, x+m, c+m}` |
  | B5 | `240 <= h < 300`| `{x+m, m, c+m}` |
  | B6 | `300 <= h < 360`| `{c+m, m, x+m}` |
  | B7 (`else`) | `120 <= h < 180`, or `h >= 360`, or `h` NaN | `{m, m, m}` |
* **A2 — saturation regime.** `s == 0` (`lib.c:10`) is a hard early return; the
  sign of `s` flips the sign of `c` and hence of `m` and `x`; `s` magnitude is
  otherwise unconstrained.
* **A3 — lightness regime.** `1.0f - fabsf(2.0f*l - 1.0f)` is piecewise: it
  rises on `l <= 0.5` and falls on `l >= 0.5`, is `0` exactly at `l ∈ {0, 1}`,
  and is **negative** outside `[0, 1]` — an input class the C accepts silently.
* **A4 — `fmodf` regime for `x`.** `fmodf(h/60.0f, 2)` behaves differently for
  `|h/60| < 2` (identity), `|h/60| >= 2` (needs reduction, and the quotient can
  need up to 128 bits of shifting for large exponents), `h/60` subnormal, and
  `h/60` non-finite (libm domain error). This axis is *independent* of A1, and
  it is the axis where the two libraries run **different code**
  (glibc `fmodf` vs statically linked `compiler_builtins` `fmodf`, see
  `SYMBOLS.md`), so it is enumerated separately.
* **A5 — bit-pattern class of each component,** independently: `+0`, `-0`,
  subnormal, normal, `FLT_MAX`, `±Inf`, quiet NaN, signalling NaN. These are the
  "element type / boundary value" shapes for a `float`-only API.
* **A6 — pointer relationship / buffer shape.** `dest` and `src` may be
  disjoint, fully aliased, or partially overlapping at ±1, ±2 floats; `dest` may
  sit at an arbitrary offset inside a larger allocation. The C reads all three
  inputs into locals before its first store, so aliasing must be lossless.
* **A7 — call sequencing.** Stateless ⇒ any interleaving of calls (and of the C
  and the Rust library) must give identical results; a translation that cached
  anything in a `static` would show up here.

Every row below is a combination of these axes that the C treats differently.
Each row is exercised with **many randomized inputs** (fixed-seed PCG32, seed
noted in the test) — never a single hand-picked value — and the two `.so`s'
outputs are compared as raw `u32` bit patterns, together with canary words
around `dest`.

## The table

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| **Hue-sector rows — A1 × randomized A2/A3** ||||
| C1  | `hsl_to_rgb` | B1: `h` uniform in `[0, 60)`; `s` uniform in `(0, 1]`; `l` uniform in `[0, 1]` | [x] |
| C2  | `hsl_to_rgb` | B2: `h` uniform in `[60, 120)`; `s ∈ (0,1]`; `l ∈ [0,1]` | [x] |
| C3  | `hsl_to_rgb` | B7 via the typo hole: `h` uniform in `[120, 180)`; `s ∈ (0,1]`; `l ∈ [0,1]` — must be grey `{m,m,m}` | [x] |
| C4  | `hsl_to_rgb` | B4: `h` uniform in `[180, 240)`; `s ∈ (0,1]`; `l ∈ [0,1]` | [x] |
| C5  | `hsl_to_rgb` | B5: `h` uniform in `[240, 300)`; `s ∈ (0,1]`; `l ∈ [0,1]` | [x] |
| C6  | `hsl_to_rgb` | B6: `h` uniform in `[300, 360)`; `s ∈ (0,1]`; `l ∈ [0,1]` | [x] |
| C7  | `hsl_to_rgb` | B7 via overflow: `h` uniform in `[360, 1e6)`; `s ∈ (0,1]`; `l ∈ [0,1]` | [x] |
| C8  | `hsl_to_rgb` | B3: `h` uniform in `(-1e6, 0)`; `s ∈ (0,1]`; `l ∈ [0,1]` — the branch reachable only through the typo | [x] |
| **Exact-boundary rows — A1 boundaries** ||||
| C9  | `hsl_to_rgb` | `h` exactly one of `{-0.0, +0.0, 60, 120, 180, 240, 300, 360}`; `s`,`l` randomized in `(0,1]`/`[0,1]` | [x] |
| C10 | `hsl_to_rgb` | `h = nextafterf(b, ±Inf)` for every boundary `b` above (one ULP either side, 16 hues); `s`,`l` randomized | [x] |
| **Lightness-regime rows — A3 × random hue over all 7 outcomes** ||||
| C11 | `hsl_to_rgb` | `l` uniform in `(-1e3, 0)` (negative ⇒ `1-\|2l-1\|` negative ⇒ negative chroma); `h` random over all sectors; `s ∈ (0,1]` | [x] |
| C12 | `hsl_to_rgb` | `l` exactly `+0.0` and exactly `-0.0` ⇒ `c = 0*s` ⇒ `c = ±0`; `h` random, `s ∈ (0,1]` | [x] |
| C13 | `hsl_to_rgb` | `l` uniform in `(0, 0.5)`; `h` random; `s ∈ (0,1]` | [x] |
| C14 | `hsl_to_rgb` | `l` exactly `0.5` ⇒ `c == s`, `m = 0.5 - 0.5c`; `h` random; `s ∈ (0,1]` | [x] |
| C15 | `hsl_to_rgb` | `l` uniform in `(0.5, 1)`; `h` random; `s ∈ (0,1]` | [x] |
| C16 | `hsl_to_rgb` | `l` exactly `1.0` ⇒ `c = 0*s`; `h` random; `s ∈ (0,1]` | [x] |
| C17 | `hsl_to_rgb` | `l` uniform in `(1, 1e3)`; `h` random; `s ∈ (0,1]` | [x] |
| C18 | `hsl_to_rgb` | `l` drawn from the special pool `{±0, ±FLT_TRUE_MIN, ±FLT_MIN, ±FLT_MAX, ±Inf, qNaN, sNaN}`; `h` random; `s ∈ (0,1]` | [x] |
| **Saturation-regime rows — A2** ||||
| C19 | `hsl_to_rgb` | `s = +0.0` exactly (early-return path) × `h` random over all 7 outcomes × `l` from the special pool — proves the early return copies `l` verbatim regardless of `h` | [x] |
| C20 | `hsl_to_rgb` | `s = -0.0` exactly (also `== 0` in C) × random `h`,`l` | [x] |
| C21 | `hsl_to_rgb` | `s` uniform in `(0, 1)`; `h` random; `l ∈ [0,1]` | [x] |
| C22 | `hsl_to_rgb` | `s` exactly `1.0`; `h` random; `l ∈ [0,1]` | [x] |
| C23 | `hsl_to_rgb` | `s` uniform in `(1, 1e6)` (over-saturated); `h` random; `l ∈ [0,1]` | [x] |
| C24 | `hsl_to_rgb` | `s` uniform in `(-1e6, 0)` (negative saturation ⇒ `c` sign flipped); `h` random; `l ∈ [0,1]` | [x] |
| C25 | `hsl_to_rgb` | `s` from the special pool `{±FLT_TRUE_MIN, ±FLT_MIN, ±FLT_MAX, ±Inf, qNaN, sNaN}`; `h` random; `l ∈ [0,1]` | [x] |
| **`fmodf` regime rows — A4** ||||
| C26 | `hsl_to_rgb` | `\|h/60\| < 2` (`h ∈ (-120, 120)`, `fmodf` is the identity); random `s`,`l` | [x] |
| C27 | `hsl_to_rgb` | `\|h/60\| >= 2` with the exponent of `h` drawn uniformly from `[2^-20, 2^40]` (reduction loop runs); random `s`,`l` | [x] |
| C28 | `hsl_to_rgb` | `h` with a *uniformly random exponent over the whole range* `2^-149 … 2^127` and random mantissa, both signs (worst case for the `fmodf` reduction); random `s`,`l` | [x] |
| C29 | `hsl_to_rgb` | `h` subnormal (`\|h\| <= FLT_MIN`, so `h/60` is subnormal or `±0`); random `s`,`l` | [x] |
| C30 | `hsl_to_rgb` | `h ∈ {+Inf, -Inf, qNaN, sNaN, ±FLT_MAX}` (libm domain error in `fmodf`); random `s`,`l` | [x] |
| **Buffer-shape / aliasing rows — A6** ||||
| C31 | `hsl_to_rgb` | disjoint `dest`/`src`, `dest` embedded in an 11-float canary buffer at offset 4; fully random inputs — asserts no OOB write | [x] |
| C32 | `hsl_to_rgb` | `dest == src` (full aliasing); fully random inputs | [x] |
| C33 | `hsl_to_rgb` | `dest == src + 1` (partial overlap, forward); fully random inputs | [x] |
| C34 | `hsl_to_rgb` | `dest == src - 1` (partial overlap, backward); fully random inputs | [x] |
| C35 | `hsl_to_rgb` | `dest == src + 2` and `dest == src - 2`; fully random inputs | [x] |
| C36 | `hsl_to_rgb` | `dest` at every offset `0..8` inside a 16-float allocation (varies 16-byte alignment class); fully random inputs | [x] |
| **Sequencing / statelessness rows — A7** ||||
| C37 | `hsl_to_rgb` | 4096 calls in one sequence per library, then the same 4096 interleaved C/Rust — asserts identical results in both orders (no hidden state, no cross-talk) | [x] |
| C38 | `hsl_to_rgb` | same input repeated 32× in a row must give the same answer every time, for inputs drawn from the special pool | [x] |
| **Whole-space fuzz rows — A5 cross-product** ||||
| C39 | `hsl_to_rgb` | `h`, `s`, `l` each an *independent uniform random 32-bit pattern* (covers every class of A5 in every combination, incl. all-NaN, all-Inf, mixed) — 200 000 cases | [x] |
| C40 | `hsl_to_rgb` | structured cross-product: each of `h`, `s`, `l` independently drawn from {special pool, uniform-in-plausible-range, random bits, random-exponent} = 4³ generator combinations × many samples | [x] |
| C41 | `hsl_to_rgb` | exhaustive over the interesting hue grid: every `h` in `-720 … +1080` step `0.25` (7201 hues) × randomized `s`,`l` — walks every sector boundary densely | [x] |
| C42 | `hsl_to_rgb` | exhaustive-by-bits sweep: all 2^16 patterns of the high 16 bits of `h` with a fixed low half, × randomized `s`,`l` — sweeps every exponent/sign combination of the hue | [x] |

## Addendum — the axis the first pass of this table missed

`CONFIGS.md` originally enumerated only the axes that affect the *returned
values*. Phase C turned up a further axis that a real consumer can observe and
that the C branches on differently from a naive Rust translation:

* **A8 — the floating-point status word the call leaves behind.** The C's hue
  dispatch uses `comiss` (the *signalling* compare, which raises `FE_INVALID`
  even for a quiet NaN), and its `addss`/`mulss`/`divss` raise `FE_INVALID` for a
  signalling-NaN operand. A caller can read that with `fetestexcept`, or turn it
  into a trap with `feenableexcept`. See `ERRORS.md` rows E21/E22.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C43 | `hsl_to_rgb` | `feclearexcept` before / `fetestexcept` after, with a **quiet** NaN hue (every payload/sign) — the C raises `FE_INVALID`, `ucomiss` would not | [x] |
| C44 | `hsl_to_rgb` | ditto with a **signalling** NaN in each of `h`, `s`, `l`, and in every pair/triple — the first arithmetic instruction touching it must raise `FE_INVALID` | [x] |
| C45 | `hsl_to_rgb` | ditto for invalid-from-valid input: `0*Inf` chroma, `Inf-Inf` midpoint, `fmodf(±Inf,2)` domain error | [x] |
| C46 | `hsl_to_rgb` | ditto for `FE_OVERFLOW` / `FE_UNDERFLOW` / `FE_INEXACT` / no-flag-at-all cases, incl. the `s == 0` early return (which must raise **nothing**) | [x] |
| C47 | `hsl_to_rgb` | ditto over randomized whole-space inputs and the full `specials^3` cross-product | [x] |
| **Exhaustive rows (`tests/exhaustive.rs`, driven to completion by `sweep.sh`)** ||||
| C48 | `hsl_to_rgb` | **all 2^32** bit patterns of `h` with `s=1, l=0.5` (so `c=1, m=0` and the three stores read out the branch and `x` directly) | [x] |
| C49 | `hsl_to_rgb` | **all 2^32** bit patterns of `h` with `s=0.7, l=0.3` (both inexact, so the `x = c * term` product rounds) | [x] |
| C50 | `hsl_to_rgb` | **all 2^32** bit patterns of `h` with `s=+Inf, l=0` (so `c`/`m`/`x` are NaN — NaN propagation for every possible hue) | [x] |
| C51 | `hsl_to_rgb` | **all 2^32** bit patterns of `s` (fixed `h`, `l`) | [x] |
| C52 | `hsl_to_rgb` | **all 2^32** bit patterns of `l` (fixed `h`, `s`) | [x] |

Rows C48–C52 matter because `h` is the only input that reaches `fmodf`, and
`fmodf` is the single place where the two shared objects run genuinely different
machine code (glibc's `fmodf` vs the one `compiler_builtins` links in
statically — see `SYMBOLS.md`). Sampling cannot establish that two different
implementations agree; enumeration can.

## Feature combinations

`Cargo.toml` has no `[features]` section and `src/lib.rs` contains no
`#[cfg(feature = ...)]`, so there is exactly **one** feature combination. It is
nevertheless exercised through both spellings (`default` and
`--no-default-features`) and both cargo profiles (`dev` and `release`) by
`verify.sh`, and *every* differential assertion is made against **both** the
`debug` and the `release` build of the Rust `cdylib` simultaneously
(`common::rust_libs()` loads both), because they are different codegen and
`release` additionally sets `panic = "abort"`.
