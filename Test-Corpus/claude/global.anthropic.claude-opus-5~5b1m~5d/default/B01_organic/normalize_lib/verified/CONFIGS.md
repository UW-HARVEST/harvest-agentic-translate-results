# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Entry points (the FULL public API)

`c_src/include/lib.h` is one line and declares exactly one function:

```c
void normalize(float *dest, const float *src, int size);
```

There is **no** convenience/one-shot wrapper layered over a lower-level API, no
context/handle object, no init/finish pair, and no separately-exported helper.
`normalize` *is* the lowest-level entry point, so every row below drives it
directly through the `.so` export.

## Axes the C code actually branches on (derived from the source)

| axis | where the C branches on it | values enumerated |
|------|---------------------------|-------------------|
| **A. `size`** (loop trip count, twice; also the `memset` length) | `for (i = 0; i < size; i++)` ×2, `size * sizeof(float)` | 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 256, 1024, 4093 — every residue class mod 2/4/8/16 so that any auto-vectorised prologue/tail in the Rust build is forced to reproduce the strictly sequential C accumulation |
| **B. aliasing of `dest` / `src`** | `else if (dest != src)` (pointer compare) **and** the read-after-write in loop #2 | disjoint; `dest == src` (exact in-place); forward overlap `dest = src + k`; backward overlap `dest = src - k` |
| **C. branch chosen by `sum > 0.0f`** | `if (sum > 0.0f)` | normalize branch (`sum` finite > 0, or `sum == +inf`); zero-fill branch (`sum == +0.0f`, `sum` NaN) — the degenerate side is tabulated in `ERRORS.md` |
| **D. value regime of `src`** (decides rounding of the `f32` accumulator, and whether the scale is exact) | `sum += src[i]*src[i]`, `1.0f/sqrtf(sum)`, `src[i]*sum` | see the 11 distributions below |
| **E. buffer alignment / start offset** | not branched on by C (scalar, unaligned-safe), but *is* branched on by any vectorised Rust codegen | float offset 0, 1, 2, 3 from a 16-byte-aligned base |

### Value distributions used for axis D

| id | distribution |
|----|--------------|
| `Unit` | uniform in `[-1, 1)` |
| `Wide` | uniform mantissa × random exponent in `2^[-40, 40]` (extreme dynamic range → accumulation order matters) |
| `FiniteBits` | uniform random 32-bit patterns, rejecting only NaN/inf (the most aggressive value fuzz) |
| `Pow2` | exact signed powers of two in `2^[-12, 12]` (sum and scale exactly representable → results must be exact) |
| `Dominant` | one element of magnitude `~1e18` and the rest `~1e-18` (the tail is swallowed; order-sensitive) |
| `AllEqual` | every element the same random value |
| `Subnormal` | random subnormals mixed with random normals |
| `OneHot` | a single random non-zero element, all other elements `+0.0` (result is exactly `±1.0`) |
| `SignedZeros` | `±0.0` mixed with random non-zero values (checks sign propagation through `src[i]*scale`) |
| `SumIsOne` | values scaled so `sum` is exactly `1.0f` → scale exactly `1.0f` → `dest` must be a bit copy of `src` |
| `Tiny` | uniform in `[-1e-20, 1e-20)`, so squares underflow but `sum` may still be > 0 |

## Configuration table

Every row is run with **many randomized inputs** (seeded `SplitMix64`, fixed
seed per row so runs are reproducible), across the full `size` list of axis A
unless the row names specific sizes. Outputs are compared as raw `u32` bit
patterns, and 8 guard floats on each side of `dest` are checked to be
untouched, so an over- or under-write is caught too.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `normalize` | disjoint buffers, `Unit`, all sizes of axis A, offset 0 | [x] |
| 2 | `normalize` | disjoint, `Unit`, all sizes, offsets 1/2/3 (misaligned start) | [x] |
| 3 | `normalize` | disjoint, `Wide`, all sizes, offset 0 | [x] |
| 4 | `normalize` | disjoint, `Wide`, all sizes, offsets 1/2/3 | [x] |
| 5 | `normalize` | disjoint, `FiniteBits`, all sizes, offset 0 | [x] |
| 6 | `normalize` | disjoint, `FiniteBits`, all sizes, offsets 1/2/3 | [x] |
| 7 | `normalize` | disjoint, `Pow2`, all sizes (exact arithmetic path) | [x] |
| 8 | `normalize` | disjoint, `Dominant`, all sizes (order-sensitive accumulation) | [x] |
| 9 | `normalize` | disjoint, `AllEqual`, all sizes | [x] |
| 10 | `normalize` | disjoint, `Subnormal`, all sizes | [x] |
| 11 | `normalize` | disjoint, `OneHot`, all sizes (result exactly `±1.0`) | [x] |
| 12 | `normalize` | disjoint, `SignedZeros`, all sizes (sign of zero in output) | [x] |
| 13 | `normalize` | disjoint, `SumIsOne`, all sizes (scale exactly `1.0f`, `dest` == bit copy of `src`) | [x] |
| 14 | `normalize` | disjoint, `Tiny`, all sizes (squares underflow; both branches reachable) | [x] |
| 15 | `normalize` | **in-place** `dest == src`, `Unit`, all sizes, offset 0 | [x] |
| 16 | `normalize` | in-place `dest == src`, `Unit`, all sizes, offsets 1/2/3 | [x] |
| 17 | `normalize` | in-place `dest == src`, `Wide`, all sizes | [x] |
| 18 | `normalize` | in-place `dest == src`, `FiniteBits`, all sizes | [x] |
| 19 | `normalize` | in-place `dest == src`, `Pow2`, all sizes | [x] |
| 20 | `normalize` | in-place `dest == src`, `Dominant`, all sizes | [x] |
| 21 | `normalize` | in-place `dest == src`, `Subnormal`, all sizes | [x] |
| 22 | `normalize` | in-place `dest == src`, `SignedZeros`, all sizes | [x] |
| 23 | `normalize` | **forward overlap** `dest = src + 1`, `Unit`, all sizes ≥ 2 | [x] |
| 24 | `normalize` | forward overlap `dest = src + k`, `k` random in `1..size`, `Unit`, all sizes ≥ 2 | [x] |
| 25 | `normalize` | forward overlap `dest = src + k`, `Wide`, all sizes ≥ 2 | [x] |
| 26 | `normalize` | forward overlap `dest = src + k`, `FiniteBits`, all sizes ≥ 2 | [x] |
| 27 | `normalize` | **backward overlap** `dest = src - 1`, `Unit`, all sizes ≥ 2 | [x] |
| 28 | `normalize` | backward overlap `dest = src - k`, `k` random in `1..size`, `Wide`, all sizes ≥ 2 | [x] |
| 29 | `normalize` | backward overlap `dest = src - k`, `FiniteBits`, all sizes ≥ 2 | [x] |
| 30 | `normalize` | disjoint, `Unit`, `size` **larger than the buffer's logical length** never happens — instead: `size` smaller than the allocated buffer, so trailing floats must be left untouched (partial-write check), all sizes | [x] |
| 31 | `normalize` | disjoint, mixture of `Unit` and exact `±1.0`/`±2.0` so `sum` is an exact integer and `sqrtf` is exact, all sizes | [x] |
| 32 | `normalize` | disjoint, values crafted so `sum` overflows to `+inf` for large `size` only (`~1e19` each) — the same buffer flips branch as `size` grows, all sizes | [x] |
| 33 | `normalize` | in-place `dest == src`, `SumIsOne` (scale exactly `1.0f`; buffer must come back bit-identical) | [x] |
| 34 | `normalize` | disjoint, `size` swept over **every** value `0..=300` with `Unit` data (exhaustive small-trip-count sweep, no gaps) | [x] |
| 35 | `normalize` | in-place, `size` swept over every value `0..=300` with `Wide` data | [x] |
| 36 | `normalize` | disjoint, large `size` (4096, 16384, 65536) with `Unit` and `Wide` (long-loop / vectoriser-eligible shapes) | [x] |

## Feature combinations (Phase D)

`translation/Cargo.toml` has no `[features]` table, so the complete set of
configurations is:

| combo | command |
|-------|---------|
| default (empty) | `cargo test --release` |
| `--no-default-features` | `cargo test --release --no-default-features` |

`run_all.sh` runs the whole suite under both, in both the `dev` and `release`
profiles (release matters: it is the profile that lets LLVM auto-vectorise the
Rust loops, which is exactly where a reassociated FP accumulation would show
up as a divergence from the scalar C).

## Results

All 36 rows pass. `tests/valid_paths.rs` contains 37 `#[test]`s (36 rows + a
mixed 4000-iteration fuzz sweep that randomises distribution x size x offset x
aliasing together). Every row runs many seeded-random inputs, and every
comparison is on raw `u32` bit patterns over the *whole* region including the
`GUARD` sentinels, so over-writes, under-writes and `+0.0`/`-0.0` or NaN-payload
differences all fail the test.

### Why the release profile matters (confirmed in the disassembly)

`objdump -d target/release/libnormalize_lib.so` shows that LLVM:

* unrolls the accumulation loop 4x but keeps **one** serial dependency chain
  (`addss %xmm0,%xmm1` -> `addss %xmm1,%xmm2` -> ...), i.e. it does **not**
  reassociate — matching the C's strictly sequential `float` accumulator;
* uses `ucomiss` + `jbe` for `sum > 0.0f`, so NaN (unordered) takes the
  zero-fill branch exactly like the C's `comiss` + `jbe`;
* uses `sqrtss` + `divss` for `1.0f / sqrtf(sum)`;
* **vectorises the store loop** (`mulps`, 2x unrolled) behind a runtime overlap
  guard (`size < 8` or `(unsigned)(dest - src) < 32` -> scalar fallback).

That last point is the reason rows 23-29 exist and are randomised over the
shift distance: forward overlaps with `dest - src >= 32` bytes take the
vectorised path while smaller ones take the scalar path, and both must
reproduce the C's read-after-write order. They do.

### C optimisation levels

The suite is additionally run against the C library rebuilt at `-O1`, `-O2`,
`-O3` and `-Os` (step 5 of `run_all.sh`); all agree bit-for-bit with both Rust
artifacts.

`-O3 -march=native` is the one C configuration that differs, and the difference
is in the **C**, not the Rust: `-march=native` enables FMA and gcc's default
`-ffp-contract=fast` then contracts `sum += src[i]*src[i]` into three `vfmadd`
instructions, which round once per iteration instead of twice. No single Rust
implementation can be bit-identical to both the contracted and the
non-contracted C, and the reference build in the task instructions
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, no `CMAKE_BUILD_TYPE`) emits
no FMA — so the translation correctly targets the non-contracted semantics.
(If a contracted C build ever became the reference, the one-line change would be
`sum = v.mul_add(v, sum)`.)
