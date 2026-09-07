# CONFIGS.md — Phase B configuration surface table

## Mechanical derivation of the axes

Grepped from the C source, not guessed:

* **Runtime options / modes / flags: NONE.** The public header declares exactly
  one function and no setters, no context/handle struct, no global state, no
  environment lookups. `grep -n '#if\|#define\|#ifdef' src/lib.c include/lib.h`
  → no matches, so there are no compile-time configuration branches either.
  `translation/Cargo.toml` declares no `[features]`, so there is one build
  configuration.
* **Public entry points: exactly one — `hsv_to_rgb`.** It *is* the lowest-level
  entry point; there is no convenience wrapper above it and no internal helper
  below it. `nm -D` confirms a single exported symbol (see `SYMBOLS.md`). So
  "test the low-level API, not just the wrappers" collapses to "test
  `hsv_to_rgb` directly", which every row below does.
* **Input shapes the code actually special-cases** (the only branches in the
  function, lines 12 and 24–55):
  1. `if (s == 0)` → achromatic early-return path vs. chromatic path.
  2. `switch ((int)floorf(h/60))` → 6 distinct arms: `0, 1, 2, 3, 4, default`.
     `default` is reachable three structurally different ways: `i >= 5`,
     `i < 0`, and `i == INT_MIN` from the undefined out-of-range cast.
  3. IEEE-754 value class of each of `h`, `s`, `v` — the arithmetic
     (`h/60`, `floorf`, `h - i`, `v*(1-s)`, `v*(1-s*f)`, `v*(1-s*(1-f))`) is
     value-dependent, so normal / zero / `-0.0` / subnormal / huge / `±inf` /
     NaN each traverse different rounding, overflow and NaN-propagation
     behaviour with the same instruction sequence.
  4. Pointer shape: `dest` and `src` disjoint vs. fully aliased vs. partially
     overlapping. The C loads all three inputs into locals *before* any store,
     so aliasing has a defined observable result the Rust must reproduce.
* **Byte order / element width / element count: fixed.** `float` (IEEE binary32,
  native LE) and exactly 3 elements, both hard-coded. No count parameter, no
  stride, no format selector — so those axes have a single value each and are
  folded into every row rather than being cross-producted.

The table is the cross-product of (achromatic | each `switch` arm) × (value
class of `h`/`s`/`v`) × (pointer aliasing), pruned to the combinations the C
actually distinguishes. Every row is driven with **many randomized inputs from a
fixed seed** (`SEED = 0x5EED_1234_ABCD_0001`, SplitMix64) plus its hand-pinned
boundary values, and asserted **bit-exact** (`to_bits()`), so `-0.0` vs `+0.0`
and distinct NaN payloads are distinguished rather than glossed over by `==`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsv_to_rgb` | achromatic: `s = +0.0`, `v` = randomized finite normals, `h` randomized normals (must be ignored) | [x] |
| 2 | `hsv_to_rgb` | achromatic: `s = -0.0` (branch still taken), `v`/`h` randomized | [x] |
| 3 | `hsv_to_rgb` | achromatic: `s = ±0.0`, `v ∈ {+0.0, -0.0, 1.0, FLT_MIN subnormal, FLT_MAX, +inf, -inf, NaN}` | [x] |
| 4 | `hsv_to_rgb` | achromatic: `s = ±0.0`, `h ∈ {NaN, ±inf, ±FLT_MAX, ±0.0, randomized}` — the ignored-input axis | [x] |
| 5 | `hsv_to_rgb` | chromatic sector 0 (`i = 0`): `h ∈ [0,60)` randomized, `s ∈ (0,1]`, `v ∈ [0,1]` randomized | [x] |
| 6 | `hsv_to_rgb` | chromatic sector 1 (`i = 1`): `h ∈ [60,120)` randomized, `s`/`v` randomized in nominal range | [x] |
| 7 | `hsv_to_rgb` | chromatic sector 2 (`i = 2`): `h ∈ [120,180)` randomized, `s`/`v` randomized | [x] |
| 8 | `hsv_to_rgb` | chromatic sector 3 (`i = 3`): `h ∈ [180,240)` randomized, `s`/`v` randomized | [x] |
| 9 | `hsv_to_rgb` | chromatic sector 4 (`i = 4`): `h ∈ [240,300)` randomized, `s`/`v` randomized | [x] |
| 10 | `hsv_to_rgb` | chromatic `default` arm via `i = 5`: `h ∈ [300,360)` randomized, `s`/`v` randomized | [x] |
| 11 | `hsv_to_rgb` | chromatic `default` arm via `i >= 6`: `h ∈ [360, 1e6)` randomized (unwrapped hue), `s`/`v` randomized | [x] |
| 12 | `hsv_to_rgb` | chromatic `default` arm via `i < 0`: `h ∈ (-1e6, 0)` randomized (negative hue), `s`/`v` randomized | [x] |
| 13 | `hsv_to_rgb` | chromatic, exact sector boundaries: `h ∈ {0,60,120,180,240,300,360,420,-60,-120}` exactly ⇒ `f = 0` ⇒ `q = v`, `t = v*(1-s)` | [x] |
| 14 | `hsv_to_rgb` | chromatic, one ULP either side of every boundary: `nextafter(k*60, ±inf)` for `k = 0..7` ⇒ adjacent-arm selection and `f` near 0 / near 1 | [x] |
| 15 | `hsv_to_rgb` | chromatic, `s = 1.0` exactly ⇒ `p = v*0 = 0` (or NaN when `v` is `±inf`), across all 6 arms | [x] |
| 16 | `hsv_to_rgb` | chromatic, `s` = smallest subnormal / tiny (`1e-45`, `1e-40`, `FLT_EPSILON`) ⇒ `1 - s` rounds to `1.0`, across all 6 arms | [x] |
| 17 | `hsv_to_rgb` | chromatic, `v = +0.0` and `v = -0.0` with `s != 0` ⇒ sign-of-zero propagation through `v*(...)` | [x] |
| 18 | `hsv_to_rgb` | chromatic, `v` subnormal / near-underflow (`1e-45`, `1e-38`) ⇒ gradual underflow in `p`,`q`,`t` | [x] |
| 19 | `hsv_to_rgb` | chromatic, `v` huge (`FLT_MAX`, `1e38`) with `s > 1` ⇒ overflow of `p`,`q`,`t` to `±inf` | [x] |
| 20 | `hsv_to_rgb` | chromatic, `s > 1` (`1.5`, `2.0`, `1e30`, `+inf`) — unclamped, across all 6 arms | [x] |
| 21 | `hsv_to_rgb` | chromatic, `s < 0` (`-0.5`, `-1e30`, `-inf`) — unclamped, across all 6 arms | [x] |
| 22 | `hsv_to_rgb` | chromatic, `h` out of `int` range so `(int)floorf(h/60)` is the UB cast: `h ∈ {±1e30, ±FLT_MAX, ±2^31*60, nextafter(2^31,±inf)*60}` | [x] |
| 23 | `hsv_to_rgb` | chromatic, NaN in each position independently and in every combination: `h`/`s`/`v` ∈ {NaN, quiet-NaN with non-default payload, value} | [x] |
| 24 | `hsv_to_rgb` | chromatic, `±inf` in each position independently and in combination ⇒ `inf*0 = NaN`, `inf-inf = NaN` paths | [x] |
| 25 | `hsv_to_rgb` | pointer shape: `dest == src` (full aliasing) — C loads all 3 inputs before storing, so output is defined; randomized inputs across all 6 arms | [x] |
| 26 | `hsv_to_rgb` | pointer shape: partial overlap `dest = src + 1` and `dest = src - 1` within one buffer, randomized inputs | [x] |
| 27 | `hsv_to_rgb` | pointer shape: `dest`/`src` disjoint but adjacent, and `src` in read-only-shaped storage (baseline for rows 25–26) | [x] |
| 28 | `hsv_to_rgb` | unconstrained property sweep: all three inputs are **fully random 32-bit patterns** (every IEEE class, incl. signalling NaNs, subnormals, huge values), 200 000 iterations, fixed seed | [x] |
| 29 | `hsv_to_rgb` | nominal-consumer property sweep: `h ∈ [0,360)`, `s ∈ [0,1]`, `v ∈ [0,1]`, 200 000 iterations, fixed seed — the real-world usage distribution | [x] |
| 30 | `hsv_to_rgb` | wide-hue property sweep: `h ∈ [-1e7, 1e7]`, `s ∈ [-2,3]`, `v ∈ [-1e5,1e5]`, 200 000 iterations, fixed seed — dense coverage of arm selection × sign combinations | [x] |
| 31 | `hsv_to_rgb` | `floorf` implementation-parity axis: the C `.so` imports `floorf@GLIBC_2.2.5` while the Rust `.so` resolves `floorf` to a **local** `compiler_builtins` libm copy (`nm` shows `t floorf`). Strided sweep over all 2^32 hue bit patterns (prime stride 4093, >1.04e6 samples, every exponent) + 300 000 random hue bit patterns + all integer degrees in `[-5000,5000]` | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` contains only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares no `[[bin]]` and
has no `src/main.rs`. The project builds **no binary executable**, so the
"compare C and Rust stdout byte-for-byte" item is structurally N/A. Verified:

```
$ grep -c add_executable c_src/CMakeLists.txt   # 0
$ ls translation/src/main.rs translation/src/bin 2>&1  # no such file or directory
```

## Optimization-level robustness of the reference

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference `.so` this
suite compares against is built at `-O0`. Because rows 22–24 depend on
NaN-payload propagation, which is sensitive to SSE operand order and therefore
to code generation, the Rust was additionally cross-checked against the same C
source compiled at `-O1`, `-O2`, `-O3` and `-Os`, over 2 000 000 fully random
32-bit input triples each (10 000 000 comparisons in total):

```
C(-O0, reference) vs Rust : 0 mismatches / 2000000
C(-O1)            vs Rust : 0 mismatches / 2000000
C(-O2)            vs Rust : 0 mismatches / 2000000
C(-O3)            vs Rust : 0 mismatches / 2000000
C(-Os)            vs Rust : 0 mismatches / 2000000
```

So the translation is not tuned to one particular code generation of the C.

## Running the matrix

```
translation/scripts/verify_matrix.sh
```

builds the C reference, enumerates the cargo feature combinations from
`Cargo.toml` (currently none, so `--no-default-features`, default and
`--all-features`), builds the cdylib in BOTH profiles for each, and runs every
phase under `cargo test` and `cargo test --release`. The harness loads whichever
of `target/{release,debug}/libhsv_to_rgb_lib.so` exist and compares each one
against C, so both codegen profiles are covered in a single run.
