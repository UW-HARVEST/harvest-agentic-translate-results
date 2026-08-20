# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Build-time configuration axes

| axis | source | values |
|---|---|---|
| Cargo features | `Cargo.toml` has **no `[features]` section** and no optional dependencies | exactly one combination: *(none)* — i.e. `--no-default-features` == default |
| C preprocessor | `grep -c '#if\|#ifdef\|#ifndef\|#define' c_src/src/lib.c c_src/include/lib.h` → **0** (only `#include "lib.h"`) | exactly one configuration |
| CMake options | `c_src/CMakeLists.txt` declares no `option()` / no `CMAKE_BUILD_TYPE` | one configuration (gcc `-O0`) |
| Rust profile | `dev` and `release` (`panic = "abort"`, overflow checks off) | both `.so`s are loaded and compared in every test |

So there is a single build configuration; all rows below are **runtime**
configuration rows, and every one is run against both the debug and the release
Rust `cdylib`.

## Public entry points (complete)

`c_src/include/lib.h` exposes exactly one, and it *is* the lowest-level entry
point (there is no convenience wrapper layer to skip):

| entry point | signature |
|---|---|
| `ldexp_q2` | `float ldexp_q2(float y, int exp_q2)` |

## Runtime branch axes (derived from the C control flow)

1. **Clamp branch** `((30*4) > exp_q2 ? exp_q2 : 120)` → `exp_q2 < 120` vs `exp_q2 >= 120`.
2. **Table index** `e & 3` → 4 distinct `g_expfrac[]` entries, for both positive
   and (two's-complement) negative `e`.
3. **Shift regime** `cnt = (e >> 2) & 31` (arithmetic shift, then x86 5-bit mask):
   * `e ∈ [0,120]` → `cnt = e>>2 ∈ [0,30]` → `shifted = 2^(30-cnt)` (never 0);
   * `e < 0` → `cnt = (e>>2) mod 32`, with three sub-regimes:
     `cnt == 31` → `shifted == 0`; `cnt == 0` → `shifted == 2^30`;
     otherwise `shifted == 2^(30-cnt)`. Period is 128 in `exp_q2`.
4. **Loop trip count** `while ((exp_q2 -= e) > 0)` → 1 iteration for
   `exp_q2 <= 120`, 2 for `121..240`, 3 for `241..360`, … `ceil(exp_q2/120)`.
5. **`y` value class** — sign, `±0`, subnormal, normal, `±FLT_MAX`, `±inf`,
   qNaN/sNaN with arbitrary payloads; plus the rounding class of `y * product`
   (exact, round-to-nearest-even tie, gradual underflow, flush to zero).

Rows are the pruned cross-product of axes 1–5, i.e. every combination the C
actually distinguishes. Every row is exercised with **many randomized inputs**
(`xorshift64*`, fixed seed `0x5EED_1234_ABCD_0001`), not one hand-picked value,
and asserted bit-for-bit (`f32::to_bits`) against the C `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `ldexp_q2` | `exp_q2 == 0`: 1 iteration, `e&3 == 0`, `cnt == 0`, `product == 1.0` exactly × 4096 random `y` bit patterns (all classes) | [x] |
| C2 | `ldexp_q2` | `exp_q2 ∈ {1,2,3}`: 1 iteration, `cnt == 0`, each of the 3 non-zero table entries × 4096 random `y` | [x] |
| C3 | `ldexp_q2` | `exp_q2 ∈ [4,115]` random: 1 iteration, `cnt ∈ [1,28]`, all 4 residues × random normal `y` | [x] |
| C4 | `ldexp_q2` | `exp_q2 ∈ [116,119]`: 1 iteration, `cnt == 29`, all 4 residues, `exp_q2 < 120` side of the clamp × random `y` | [x] |
| C5 | `ldexp_q2` | `exp_q2 == 120` exactly: clamp boundary (`>` not `>=`), `cnt == 30`, `shifted == 1`, `exp_q2-e == 0` → exactly 1 iteration × random `y` | [x] |
| C6 | `ldexp_q2` | `exp_q2 ∈ [121,240]` random: **2 iterations** (`e=120` then `e=exp_q2-120`), second iteration covers all 4 residues and `cnt ∈ [0,30]` × random `y` | [x] |
| C7 | `ldexp_q2` | `exp_q2 == 240` and `exp_q2 == 241`: 2 vs **3** iterations (second/third `e == 120`) × random `y` | [x] |
| C8 | `ldexp_q2` | `exp_q2 ∈ [242, 20000]` random: **3..167 iterations**, repeated multiplication accumulates → gradual underflow / flush to zero × random `y` incl. `±inf`, NaN | [x] |
| C9 | `ldexp_q2` | `exp_q2 ∈ [-4,-1]`: negative-shift regime `cnt == 31` → `shifted == 0` → `product == 0` × random `y` (signed zeros, `±inf` → QNaN) | [x] |
| C10 | `ldexp_q2` | `exp_q2 ∈ [-124,-5]` random: negative-shift regime `cnt ∈ [1,30]` (amplifying), all 4 residues × random `y` | [x] |
| C11 | `ldexp_q2` | `exp_q2 ∈ [-128,-125]`: `cnt == 0` wrap point → `product == g_expfrac[e&3]·2^30` (up to `1.0`) × random `y` | [x] |
| C12 | `ldexp_q2` | `exp_q2 ∈ [-132,-129]`: second period, `cnt == 31` again → `product == 0` × random `y` | [x] |
| C13 | `ldexp_q2` | `exp_q2` exhaustive over `[-4096, 4096]` (all 8193 values, covering 64 full mod-128 periods on the negative side and 35 loop-trip counts on the positive side) × a fixed panel of 24 `y` values spanning every float class | [x] |
| C14 | `ldexp_q2` | `exp_q2 ∈ [-2_000_000, -1]` random: deeply negative, exercises `e>>2` sign extension far past one period × random `y` | [x] |
| C15 | `ldexp_q2` | `exp_q2 == INT_MIN`, `INT_MIN+1..+3`, `INT_MIN+124..+128` (extreme negative residues & wrap points) × full `y` panel | [x] |
| C16 | `ldexp_q2` | `exp_q2 == INT_MAX`, `INT_MAX-1`, `INT_MAX-119`, `INT_MAX-120` (maximum trip count, ~1.8e7 iterations) × small `y` panel (`1.0`, `-1.0`, `±inf`, qNaN, `±0`) | [x] |
| C17 | `ldexp_q2` | `y == ±0.0` (both signs) × every `exp_q2` in the boundary panel — sign-of-zero preservation through 1..N multiplications | [x] |
| C18 | `ldexp_q2` | `y == ±inf` × every `exp_q2` in the boundary panel — includes the `inf * 0` invalid-operation combination and `inf` surviving many iterations | [x] |
| C19 | `ldexp_q2` | `y` = qNaN and sNaN with 512 random payloads (both signs) × random `exp_q2` — NaN payload/sign propagation and sNaN quieting | [x] |
| C20 | `ldexp_q2` | `y` = subnormals (min subnormal, random 23-bit-mantissa subnormals, both signs) × `exp_q2 ∈ [0,240]` — gradual underflow and round-to-nearest-even ties | [x] |
| C21 | `ldexp_q2` | `y` = `±FLT_MAX`, `±FLT_MIN` (smallest normal), `±1.0`, `±2^-149` × `exp_q2 ∈ {-128,-1,0,1,120,121,240}` — extremes of the dynamic range | [x] |
| C22 | `ldexp_q2` | Fully random fuzz: `y` = uniform random 32-bit pattern, `exp_q2` = uniform random `i32` clamped to `[-2^31, 20000]` (positive side bounded for runtime), 200 000 pairs | [x] |
| C23 | `ldexp_q2` | Fully random fuzz over the *whole* `i32` domain including the huge-trip-count half: 64 pairs with `exp_q2` uniform in `[0, INT_MAX]` | [x] |
| C24 | `ldexp_q2` | Repeat-call / statelessness: the same `(y, exp_q2)` called 3× interleaved between the two libraries — checks the `static const` table is not mutated and there is no hidden state | [x] |
| C25 | `ldexp_q2` | **Exhaustive**: all 2^23 subnormal `y` bit patterns (exponent field 0) × `exp_q2 ∈ {1,2,3,4,5,8,-1,-5,120,121}` — 84 M calls per implementation, covers every round-to-nearest-even tie in the subnormal range | [x] |
| C26 | `ldexp_q2` | **Exhaustive**: all 2^23 mantissas of the binades `exp_field ∈ {1, 126, 254, 255}` (tiny normals, unit binade, largest binade, the inf/NaN binade) × `exp_q2 ∈ {1,4,-1,-124,121}` | [x] |
| C27 | `ldexp_q2` | **Exhaustive**: all 2^23 mantissas with the sign bit set, `exp_field ∈ {0,1,127,255}` × `exp_q2 ∈ {3,-2,240}` — sign propagation through 1 and 2 iterations | [x] |

All 27 rows have a passing differential test in `tests/valid_paths.rs`.

## Harness validation (mutation testing)

Because a passing differential suite is only meaningful if it *can* fail, the
suite was re-run against deliberately broken Rust `cdylib`s injected via
`LDEXP_RUST_SO`:

| mutant | change | result |
|---|---|---|
| m1 | add `if exp_q2 <= 0 { return y; }` early return | **caught** (10+ tests fail) |
| m2 | `G_EXPFRAC[(e as usize) % 4]` instead of `& 3` | survived — proven **equivalent** (`(e as u64) % 4 == e & 3` for all `i32`, since `2^64 ≡ 0 mod 4`) |
| m3 | "fix" the shift UB with `.clamp(0, 31)` instead of the x86 5-bit mask | **caught** |
| m4 | clamp with `>=` instead of `>` | survived — proven **equivalent** (both compute `min(exp_q2,120)`; they differ only in which of two *equal* operands is selected) |
| m5 | `e / 4` (truncating) instead of `e >> 2` (floor) | **caught** |
| m7 | compute in `f64` and round once at the end | survived — proven **equivalent** over 250 M cases (the `f32`×`f32` product needs ≤ 48 bits, so it is exact in `f64`; there is no double rounding) |
| m9 | `g_expfrac[3]` literal written with fewer digits | survived — proven **equivalent** (both decimal literals round to `0x301837f0`) |
| m10 | swap `g_expfrac[1]` and `g_expfrac[2]` | **caught** |
| m11 | mask the shift count with `& 15` instead of `& 31` | **caught** |
| m12 | perturb `g_expfrac[0]` by 1 ulp (`0x30800000` → `0x30800001`) | **caught** |

Every surviving mutant was independently proven semantically equivalent to the
reference C, so there is no blind spot in the suite.
