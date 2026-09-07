# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

## Axes derived mechanically from the C source

`c_src/include/lib.h` exposes exactly one entry point, so the "full set of
public entry points, including the lowest-level ones" is a single function:

```c
void hsv_to_rgb(float *dest, const float *src);   /* the ONLY entry point */
```

There are **no runtime options, modes, flags, context/handle objects,
init/teardown calls, byte-order switches or element-type switches** — grep for
`#ifdef` / `#if` / global state in `c_src/src/lib.c` returns nothing, and the
Rust crate declares **no** `[features]` in `Cargo.toml`, so there is exactly
one feature combination (`--no-default-features` ≡ default ≡ all-features).

The axes the C code *actually branches on* are therefore purely input-shape:

* **A1 — `s == 0` predicate** (line 12): `{ s is ±0.0 → early return, s is anything else → full path }`.
  NB: `s = NaN` and `s` subnormal both take the *full* path.
* **A2 — `i = (int)floorf(h/60.0f)`, the `switch` discriminant** (line 24–55),
  6 distinguished outcomes: `{0, 1, 2, 3, 4, default}`, where `default` splits
  into sub-shapes the conversion itself distinguishes:
  `{i<0, i>=5, i==INT_MIN-from-NaN, i==INT_MIN-from-overflow}`.
* **A3 — fractional part `f = h - i`** feeding `q` and `t`: `{f == 0 exactly
  (h a multiple of 60), 0 < f < 1 generic, f near 1}`.
* **A4 — float value class of each of `h`, `s`, `v`** independently:
  `{normal, ±0.0, subnormal, ±Inf, NaN, huge (near f32 max), tiny}`.
* **A5 — `s` magnitude relative to 1** (drives sign of `1 - s`, hence sign of
  `p`, `q`, `t`): `{0<s<1 canonical, s == 1 exactly, s > 1, s < 0}`.
* **A6 — pointer shape of `dest` vs `src`** (params are **not** `restrict`):
  `{disjoint, dest == src, dest == src+1, dest == src-1, unaligned}`.

Rows below are the cross-product of A1–A6 pruned to the combinations the code
distinguishes. Every row is exercised with **many randomized inputs**
(`SEED = 0x5eed_1234`, deterministic xorshift PRNG) and compared **bit-for-bit**
(`f32::to_bits`) between the C `.so` and the Rust `.so`, both loaded through
`libloading`.

## Row table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsv_to_rgb` | A1 early-return: `s = +0.0`, `h`/`v` randomized over all normal ranges, disjoint buffers | [x] |
| 2 | `hsv_to_rgb` | A1 early-return: `s = -0.0`, randomized `h`/`v` | [x] |
| 3 | `hsv_to_rgb` | A1 early-return with `v` from the special set {±0.0, ±Inf, NaN, subnormal, f32::MAX/MIN} | [x] |
| 4 | `hsv_to_rgb` | A2 `i == 0`: `h ∈ [0,60)` randomized, `s ∈ (0,1)`, `v ∈ [0,1]` (canonical HSV) | [x] |
| 5 | `hsv_to_rgb` | A2 `i == 1`: `h ∈ [60,120)` randomized, canonical `s`,`v` | [x] |
| 6 | `hsv_to_rgb` | A2 `i == 2`: `h ∈ [120,180)` randomized, canonical `s`,`v` | [x] |
| 7 | `hsv_to_rgb` | A2 `i == 3`: `h ∈ [180,240)` randomized, canonical `s`,`v` | [x] |
| 8 | `hsv_to_rgb` | A2 `i == 4`: `h ∈ [240,300)` randomized, canonical `s`,`v` | [x] |
| 9 | `hsv_to_rgb` | A2 `default` via `i == 5`: `h ∈ [300,360)` randomized, canonical `s`,`v` | [x] |
| 10 | `hsv_to_rgb` | A2 `default` via `i >= 6`: `h ∈ [360, 1e6)` randomized (no hue wrapping in C) | [x] |
| 11 | `hsv_to_rgb` | A2 `default` via `i < 0`: `h ∈ (-1e6, 0)` randomized negative hue | [x] |
| 12 | `hsv_to_rgb` | A3 `f == 0` exactly: `h = 60*k` for k = 0..8 and k negative, randomized `s`,`v` | [x] |
| 13 | `hsv_to_rgb` | A3 `f` just below 1: `h = nextafter(60*(k+1), -inf)`, randomized `s`,`v` | [x] |
| 14 | `hsv_to_rgb` | A5 `s == 1.0` exactly (⇒ `p == 0`, sign-of-zero observable), randomized `h`,`v` | [x] |
| 15 | `hsv_to_rgb` | A5 `s > 1` (e.g. `s ∈ (1, 1e6)`) ⇒ `1-s` negative ⇒ negative `p`,`q`,`t`; randomized `h`,`v` | [x] |
| 16 | `hsv_to_rgb` | A5 `s < 0` (`s ∈ (-1e6, 0)`, excluding `-0.0`) ⇒ `1-s > 1`; randomized `h`,`v` | [x] |
| 17 | `hsv_to_rgb` | A4 `v` special {±Inf, NaN, ±0.0, subnormal, MAX} × A2 all 6 arms, `s ∈ (0,1)` randomized | [x] |
| 18 | `hsv_to_rgb` | A4 `s` special {±Inf, NaN, subnormal, MAX} × A2 all 6 arms, randomized `v` | [x] |
| 19 | `hsv_to_rgb` | A4 `h` special {±Inf, NaN, subnormal, ±f32::MAX, 2^31·60, 1e30} × randomized `s`,`v` | [x] |
| 20 | `hsv_to_rgb` | A4 fully unconstrained: all three components drawn as **uniform random 32-bit patterns** (any float incl. NaN payloads/Inf/subnormal), 200 000 iterations | [x] |
| 21 | `hsv_to_rgb` | A4 `h`,`s`,`v` drawn from a curated boundary pool (cross-product of ~30 boundary floats³ ≈ 27 000 exhaustive triples) | [x] |
| 22 | `hsv_to_rgb` | A6 `dest == src` (full alias), randomized inputs on the full path | [x] |
| 23 | `hsv_to_rgb` | A6 `dest == src` (full alias) on the `s == 0` early path | [x] |
| 24 | `hsv_to_rgb` | A6 `dest == src + 1` (forward-overlap), randomized inputs | [x] |
| 25 | `hsv_to_rgb` | A6 `dest == src - 1` (backward-overlap), randomized inputs | [x] |
| 26 | `hsv_to_rgb` | A6 unaligned `src` and `dest` (byte-offset 1/2/3 within a `u8` arena), randomized inputs | [x] |
| 27 | `hsv_to_rgb` | A6 disjoint buffers with guard canaries either side of the 3-float window (verifies exactly 3 reads / 3 writes) | [x] |
| 28 | `hsv_to_rgb` | repeated back-to-back invocations reusing the same buffers (verifies no hidden static/global state in either library) | [x] |

## Binary / driver

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no** `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`. The
project builds **no binary executable**, so the "compare C and Rust stdout"
gate is not applicable (nothing to run).

## Feature combinations

`translation/Cargo.toml` has no `[features]` table. The complete set of feature
combinations is therefore `{ default }` = `{ --no-default-features }` =
`{ --all-features }`; the suite is run under all three invocations to confirm.

## Verification results

All 28 rows are implemented as one `#[test]` each in
`tests/phase_b_configs.rs` and pass with **0 divergences**:

```
tests/phase_b_configs.rs: 28 passed; 0 failed
tests/phase_c_errors.rs:  20 passed; 0 failed; 1 ignored (child-process helper)
```

Re-run everything with `./run_all.sh` (all feature combos × both profiles) and
`./check_parity.sh` (symbol parity). Confirmed green for:

| profile | features | result |
|---------|----------|--------|
| release | default | PASS |
| release | `--no-default-features` | PASS |
| release | `--all-features` | PASS |
| dev | default | PASS |
| dev | `--no-default-features` | PASS |
| dev | `--all-features` | PASS |

### C-codegen independence

The suite was additionally replayed against the C library rebuilt at three
optimization levels (via `HARNESS_C_SO`, without modifying `c_src/`):

| C build | float→int / floor codegen | result |
|---------|---------------------------|--------|
| `-O0` | `call floorf@plt` + `cvttss2si` | PASS |
| `-O2` | inlined `roundss` + `cvttss2si` | PASS |
| `-O3` | inlined `roundss` + `cvttss2si` | PASS |

This proves the Rust `f32::floor` is bit-identical to libm `floorf` here and
that the `c_float_to_int` model of `cvttss2si` (NaN / out-of-range →
`INT_MIN`) matches the real hardware conversion the C performs.

### Harness integrity (important)

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` lib target, so an
opportunistic "load `target/release/*.so`" harness silently tests a **stale**
artifact. This was caught by a negative control: an intentional bug injected
into the `default:` arm of `src/lib.rs` was **not** detected. The harness now
shells out to `cargo build --release --lib` into a dedicated
`CARGO_TARGET_DIR` on every run; after that fix the same injected bug was
detected by 10 of the 28 rows, and the control was reverted.
