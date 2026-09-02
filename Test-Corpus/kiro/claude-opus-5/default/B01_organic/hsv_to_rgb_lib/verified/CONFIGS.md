# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axes the C code actually branches on

**Public entry points (complete set, from the header):**

| entry point | level | notes |
|-------------|-------|-------|
| `hsv_to_rgb(float *dest, const float *src)` | lowest AND only | there is no convenience wrapper and no lower layer; `floorf` is the only callee |

**Compile-time options:** none — `grep -cE '#if|#ifdef|#ifndef' c_src/src/lib.c` = 0.
`CMakeLists.txt` defines no `target_compile_definitions`.

**Runtime options / modes / flags:** none — the API takes no flag, mode, enum, or
length argument. The array length is hard-coded to 3 by the source
(`src[0..2]`, `dest[0..2]`).

**Input shapes the code special-cases** (the real configuration axes):

| axis | distinct states the C distinguishes |
|------|-------------------------------------|
| A. saturation guard (line 12) | `s == 0` (incl. `-0.0f`) vs `s != 0` |
| B. sector index `i = (int)floorf(h/60)` (line 24) | `0`, `1`, `2`, `3`, `4`, `default` (`i<0` or `i>=5`) |
| C. fractional part `f = h/60 - i` | `f == 0` exactly (hue is a multiple of 60) vs `0 < f < 1` |
| D. float class of `h` | normal, `±0.0`, subnormal, large finite, `±inf`, NaN |
| E. float class of `s` | `±0.0`, subnormal, in `(0,1)`, `1.0`, `>1`, `<0`, `±inf`, NaN |
| F. float class of `v` | `±0.0`, subnormal, in `(0,1]`, `>1`, `<0`, `±inf`, NaN |
| G. pointer relationship | disjoint, `dest == src`, `dest == src+1`, `dest == src+2` |
| H. buffer/offset alignment | `dest`/`src` at differing offsets inside a larger buffer (no alignment assumption in C beyond `float`) |

`i` is *value-dependent* on `h`, so axis B is exercised by choosing `h`; axes
A/E/F are independent of B. The table below is the pruned cross-product of the
combinations the code treats differently.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `hsv_to_rgb` | A=`s==0` (`+0.0f`), v randomized over normals — achromatic early return | [x] |
| C2 | `hsv_to_rgb` | A=`s==0` (`-0.0f`), v randomized over normals | [x] |
| C3 | `hsv_to_rgb` | A=`s==0`, F=v ∈ {`+0.0`,`-0.0`,subnormal,`±inf`,NaN,negative,`>1`} (all v classes) | [x] |
| C4 | `hsv_to_rgb` | B=0: `h` random in `[0,60)`, s,v random in `(0,1]`, C=`0<f<1` | [x] |
| C5 | `hsv_to_rgb` | B=1: `h` random in `[60,120)`, s,v random in `(0,1]` | [x] |
| C6 | `hsv_to_rgb` | B=2: `h` random in `[120,180)`, s,v random in `(0,1]` | [x] |
| C7 | `hsv_to_rgb` | B=3: `h` random in `[180,240)`, s,v random in `(0,1]` | [x] |
| C8 | `hsv_to_rgb` | B=4: `h` random in `[240,300)`, s,v random in `(0,1]` | [x] |
| C9 | `hsv_to_rgb` | B=default (`i=5`): `h` random in `[300,360)`, s,v random in `(0,1]` | [x] |
| C10 | `hsv_to_rgb` | B=default (`i>=6`): `h` random in `[360, 1e6)` (unclamped hue past one turn) | [x] |
| C11 | `hsv_to_rgb` | B=default (`i<0`): `h` random in `(-1e6, 0)` (negative hue) | [x] |
| C12 | `hsv_to_rgb` | C=`f==0`: `h` exactly at each sector boundary `{0,60,120,180,240,300,360,420,-60,-120}`, s,v randomized | [x] |
| C13 | `hsv_to_rgb` | C: `h` one ULP below/above each boundary `{60,120,180,240,300,360}` — the sector-index tipping point | [x] |
| C14 | `hsv_to_rgb` | D=`h` large finite with `\|h/60\| >= 2^31`, s,v randomized | [x] |
| C15 | `hsv_to_rgb` | D=`h` ∈ {`+inf`,`-inf`,NaN,`+0.0`,`-0.0`,min subnormal,`-`min subnormal}, s,v randomized | [x] |
| C16 | `hsv_to_rgb` | E=`s` ∈ {min subnormal, 1e-30, 0.5, `1.0`} × B=all 6 sectors | [x] |
| C17 | `hsv_to_rgb` | E=`s > 1` (up to `1e30`, unclamped) × B=all 6 sectors, v randomized | [x] |
| C18 | `hsv_to_rgb` | E=`s < 0` (down to `-1e30`, unclamped) × B=all 6 sectors, v randomized | [x] |
| C19 | `hsv_to_rgb` | E=`s` ∈ {`+inf`,`-inf`,NaN} × B=all 6 sectors, v randomized | [x] |
| C20 | `hsv_to_rgb` | F=`v` ∈ {`±0.0`, min subnormal, negative, `>1`, `1e30`} × B=all 6 sectors, s randomized in `(0,1]` | [x] |
| C21 | `hsv_to_rgb` | F=`v` ∈ {`+inf`,`-inf`,NaN} × B=all 6 sectors, s randomized in `(0,1]` | [x] |
| C22 | `hsv_to_rgb` | G=`dest == src` (in-place), randomized canonical HSV — chromatic path | [x] |
| C23 | `hsv_to_rgb` | G=`dest == src` (in-place), randomized with `s == 0` — achromatic path | [x] |
| C24 | `hsv_to_rgb` | G=`dest == src+1` and `dest == src+2` (partial overlap), randomized | [x] |
| C25 | `hsv_to_rgb` | H=`dest`/`src` at every offset pair in `0..4` of a 16-float scratch buffer, randomized; asserts neighbouring floats are NOT modified (exactly 3 writes) | [x] |
| C26 | `hsv_to_rgb` | Unrestricted fuzz: all three inputs are uniformly random 32-bit patterns reinterpreted as `f32` (covers every float class and NaN payload) | [x] |
| C27 | `hsv_to_rgb` | Canonical-range fuzz: `h ∈ [0,360)`, `s ∈ [0,1]`, `v ∈ [0,1]` — the documented happy path, dense random sweep | [x] |
| C28 | `hsv_to_rgb` | Grid sweep: `h` over `[-720, 1080]` in fine steps × `s ∈ {0, 1e-7, 0.25, 0.5, 1, 2}` × `v ∈ {0, 0.5, 1, 255}` (deterministic exhaustive-ish cross-product) | [x] |
| C29 | `hsv_to_rgb` | I=NaN **encoding** axis: dense cross-product of 14 NaN encodings (both signs × quiet/signalling × min/typical/max payload) + `±inf` + plain values across ALL THREE argument slots (`d1_nan_cross_product_all_slots`) | [x] |
| C30 | `hsv_to_rgb` | I=one NaN encoding per slot × B=all 6 sector arms × the other two slots swept over plain values (`d2_nan_per_slot_per_sector`) | [x] |
| C31 | `hsv_to_rgb` | NaN-biased fuzz: each slot is a random NaN / `±inf` / `±0` / arbitrary pattern with ~equal probability, so multi-NaN triples are common (`d3_nan_biased_fuzz`) | [x] |
| C32 | `hsv_to_rgb` | Infinity-only combinations, which generate NaNs from non-NaN operands (`inf*0`, `inf-inf` → hardware default NaN, a different mechanism from operand forwarding) (`d4_infinity_generated_nans`) | [x] |
| C33 | `hsv_to_rgb` | Regression pins for every divergence actually found and fixed (`d5_regression_pins`) | [x] |
| C34 | `hsv_to_rgb` | Cross-profile parity: the debug and release Rust `cdylib`s must agree with each other as well as with C (`d6_debug_and_release_cdylib_agree`) | [x] |

Rows C29–C34 were added *after* C26 turned out to under-sample the NaN space:
uniform 32-bit fuzzing puts a NaN in only ~0.8% of slots, so triples with two or
three NaNs are effectively never generated — and that is precisely where the
divergence below was hiding.

## Axis I — NaN encoding (added after a real divergence was found)

Float multiply and subtract are commutative/associative in *value* but not in
NaN sign-and-payload propagation: the SSE scalar instructions forward the FIRST
source operand, quieted. The order of the machine operands is not always the
order written in the C source. Adding this axis to the table is what exposed the
bug, so it is recorded as a first-class configuration axis.

## Findings

| # | divergence | root cause | fix (Rust only) |
|---|------------|------------|-----------------|
| F1 | `hsv_to_rgb([+NaN, 0.7, -NaN])` — C returned `q = 0x7FC00000`, Rust returned `0xFFC00000`. Reproduced in the **debug** profile; the release profile happened to agree, so a release-only run missed it. | `q = v * (1 - s*f)` is emitted by GCC as `mulss` with `(1 - s*f)` as the first source operand, i.e. `(1-s*f) * v`. LLVM chose the opposite operand order for the commutative `fmul` in the debug profile, so a NaN `v` won instead of a NaN `(1-s*f)`. Likewise `s * (1 - f)` is emitted as `(1-f) * s`, reversed from the source order. | Replaced the bare `*`, `-` and `/` with `c_mul` / `c_sub` / `c_div` helpers that implement the SSE "first source operand wins, quieted" rule explicitly, with the operand order taken from `objdump -d` of the reference build. The result no longer depends on LLVM's operand choice, so both profiles now match. |
| F2 | Overlapping `dest`/`src` was undefined behaviour in Rust (no observed miscompile, but `noalias` reasoning could reorder the stores ahead of the loads). | The original translation built a `&[f32]` and a `&mut [f32]` over the same memory. The C reads all three inputs into locals before its first store, so in-place conversion is legal for callers. | Switched to `ptr::read` / `ptr::write`, removing the aliasing UB while preserving read-before-write ordering. Covered by C22–C25 and E16–E17. |
| F3 | The test harness itself gave a false pass: `cargo test` does **not** rebuild a `cdylib` target, so the suite was loading a stale `.so`. An injected bug went undetected. | Cargo builds the lib target as a test harness, not as a shared object, for `cargo test`. | `tests/common/mod.rs` now refuses to run unless both `.so` files are newer than their sources (`assert_fresh`), and `scripts/` always builds before testing. Re-verified with `scripts/mutation_check.sh`. |

## Ground-truth stability

The reference `CMakeLists.txt` passes no `-O` flag. To confirm the matched
behaviour is a property of the C source rather than of one compiler setting, the
C library was also built out-of-tree (nothing in `c_src/` was modified) at `-O0`,
`-O1`, `-O2`, `-O3` and `-Ofast`, then compared over 203,375 inputs including the
dense NaN cross-product:

| C build | mismatches vs. the reference `.so` |
|---------|-----------------------------------|
| `-O0`, `-O1`, `-O2`, `-O3` | 0 |
| `-Ofast` (`-ffast-math`, deliberately changes float semantics) | 21,625 — not the ground truth |
| **Rust release `cdylib`** | **0** |

All rows are driven with many randomized inputs per row (fixed-seed
`SplitMix64`, seed `0x5EED_1234_ABCD_EF01`) and compared bit-for-bit
(`to_bits()`), not with `==`, so NaN payloads and signed zeros are covered.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the feature power set is
`{default}` = `{}`. `scripts/check_all_features.sh` enumerates it from
`cargo metadata` and runs the full suite for each element.
