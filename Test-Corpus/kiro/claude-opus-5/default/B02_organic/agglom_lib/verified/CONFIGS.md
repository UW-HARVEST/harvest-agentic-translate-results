# CONFIGS.md — configuration surface table (valid inputs)

The library has **no** runtime option struct, no init/teardown, no global mode
flag, no `#ifdef`, and no Cargo features (see `SYMBOLS.md`). Its "configuration"
axes are therefore exactly the branch selectors the C code reads out of its
arguments:

| axis | where it lives | distinct values the C branches on |
|------|----------------|-----------------------------------|
| `C2_TYPE typeA` / `typeB` | `f2`, two-level `switch` (lines 83–108) | `CIRCLE`(0) / `AABB`(1) → 4 valid dispatch pairs, each calling a *different* predicate; `AABB×CIRCLE` swaps the argument order |
| sign quadrant of `(v1, v2)` + `INT_MIN`-ness of each | `f3` (lines 110–142) | 9 reachable arms, plus the `r >= 0` vs `r < 0` post-correction and the `v2 > 0` sign of the correction |
| generator state, call count | `f4` / `cn_rnd_next` | state is **mutated in place**, so a single call and a 1000-call chain are different configurations |
| low vs high half of the word | `f5` | high 16 bits are masked away (see `ERRORS.md` E22) |
| `channels == 2` and `bitdepth == 32` | `f7` (lines 451–459) | 2×2 = 4 combinations, each activating a different subset of the three summed products |
| triangle degeneracy | `f9` | non-degenerate / collinear / coincident (`invDenom` = ±inf) |
| `h >> 10` (exponent row) | `f10` | 64 rows; `n = 0` (subnormal), `1..30`, `31` (inf/NaN), `32` (negative subnormal), `33..62`, `63` (negative inf/NaN) |
| hue sector, `s == 0` | `f11` (7 arms) / `f12` (6 `switch` arms) | see rows below |
| which channel is `max` | `f13` | `r`, `g`, `b`, plus `delta == 0` / `max == 0` |
| float class of every scalar | all of the above | normal, `±0.0`, subnormal, `±inf`, quiet NaN, signalling NaN, negative |

Every row below is driven through **both** `.so`s via `libloading`, with the
low-level entry points called directly (not only through `f2`/`agglom`), using
many seeded-random inputs per row (`SEED = 0x9E3779B97F4A7C15`), and results
compared **bit-for-bit** (`to_bits()`), not with a float tolerance.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `c2V` | random `(x, y)` incl. `±0.0`, subnormals, `±inf`, NaN payloads — verifies the 8-byte-struct-in-`xmm0` return ABI | [x] |
| C2 | `c2Maxv`, `c2Minv` | random pairs; plus every `±0.0` combination and NaN in either operand (the C uses `>`/`<` ternaries, so NaN always selects `b`) | [x] |
| C3 | `c2Clampv` | random `(a, lo, hi)`; plus inverted range `lo > hi`, `lo == hi`, NaN in each of the three | [x] |
| C4 | `c2Sub` | random pairs; plus `inf - inf`, `0 - 0`, NaN operands (payload survival order) | [x] |
| C5 | `c2Dot` | random pairs; plus `0 * inf` (→ NaN), both-operand-NaN cases where operand order decides the surviving payload | [x] |
| C6 | `c2CircletoCircle` (direct) | overlapping / disjoint / exactly touching (`d2 == r2`, strict `<` so → 0) / zero radius / negative radius / `r` such that `A.r + B.r` overflows to `inf` / NaN centre | [x] |
| C7 | `c2CircletoAABB` (direct) | circle centre inside / outside / on each face / on each corner of the AABB; inverted AABB (`min > max`, which makes `c2Clampv` return `lo`); zero-area AABB; zero and negative radius; NaN in any field | [x] |
| C8 | `c2AABBtoAABB` (direct) | fully overlapping / disjoint on each of the 4 sides / edge-touching (`B.max.x == A.min.x` → not `<` → overlap) / inverted boxes / NaN fields | [x] |
| C9 | `f2`, `typeA=CIRCLE, typeB=CIRCLE` | reinterprets both pointers as `c2Circle`; random circles + the C6 shapes | [x] |
| C10 | `f2`, `typeA=CIRCLE, typeB=AABB` | `A` is a `c2Circle` (12 B), `B` is a `c2AABB` (16 B); random + the C7 shapes | [x] |
| C11 | `f2`, `typeA=AABB, typeB=CIRCLE` | **argument-swapping arm**: C calls `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)`, i.e. `A` is read as the AABB and `B` as the circle. Random + C7 shapes | [x] |
| C12 | `f2`, `typeA=AABB, typeB=AABB` | both read as `c2AABB`; random + the C8 shapes | [x] |
| C13 | `f3`, `v1 >= 0, v2 > 0` | plain truncating division arm (`return v1/v2` — no correction) | [x] |
| C14 | `f3`, `v1 >= 0, v2 < 0, v2 != INT_MIN` | `q = -(v1/-v2), r = v1 % -v2` arm, incl. `r == 0` vs `r > 0` (`r>=0` always here → no correction) | [x] |
| C15 | `f3`, `v1 < 0, v1 != INT_MIN, v2 > 0` | `q = -((-v1)/v2), r = -((-v1)%v2)` arm; exercises **both** `r == 0` (no correction) and `r < 0` (`q - 1`) | [x] |
| C16 | `f3`, `v1 < 0, v1 != INT_MIN, v2 < 0, v2 != INT_MIN` | `q = (-v1)/(-v2), r = -((-v1)%(-v2))` arm; both `r == 0` and `r < 0` (`q + 1`) | [x] |
| C17 | `f3`, exhaustive small grid | every `(v1, v2)` in `-40..=40` × `-40..=40` — covers all arms and both correction directions densely | [x] |
| C18 | `f4`, single call | random 128-bit states incl. `{0,0}`, `{1,0}`, `{0,1}`, `{u64::MAX, u64::MAX}`; compares the returned `double` **and** the mutated `state[0]`/`state[1]` written back through the pointer | [x] |
| C19 | `f4`, 1000-call chain | same generator advanced 1000 times from a random seed; every intermediate double and the final state compared — catches state-update-order bugs a single call cannot see | [x] |
| C20 | `f5` | random `u32`; plus all 16 single-bit-in-low-half values, all 16 single-bit-in-high-half values (which must vanish), `0`, `0xFFFF`, `0xFFFF0000`, `0xFFFFFFFF` | [x] |
| C21 | `f7`, `channels == 2, bitdepth == 32` | only the `(channels==2)` products contribute and `bitdepth != 32` is 0; random `blocksize` incl. `0`, `1`, `4096`, `65535`, `0xFFFFFFFF` | [x] |
| C22 | `f7`, `channels == 2, bitdepth != 32` | the `bitdepth + 1` product activates; `bitdepth` ∈ {0,1,4,8,12,16,20,24,31,33,64,0xFFFFFFFF} × random `blocksize` | [x] |
| C23 | `f7`, `channels != 2, bitdepth == 32` | only the `channels * (channels != 2)` product; `channels` ∈ {0,1,3,4,8,255,0xFFFFFFFF} | [x] |
| C24 | `f7`, `channels != 2, bitdepth != 32` | same as C23 with the `bitdepth` axis free; includes the `channels == 0` degenerate and full random sweep over all three args (wrapping overflow) | [x] |
| C25 | `f9`, non-degenerate | random triangles + random probe point, incl. probe inside, outside, and at each vertex | [x] |
| C26 | `f9`, degenerate | `p1 == p2 == p3`; collinear `p1,p2,p3`; `p2 == p1`; zero-length `v0` or `v1` → `invDenom` is `±inf` and the result is NaN — exact NaN bit pattern compared | [x] |
| C27 | `f9`, special floats | any component `±0.0`, subnormal, `±inf`, NaN — checks which NaN payload survives through the 5 dot products and 2 final multiplies | [x] |
| C28 | `f10`, exhaustive | **all 65536** `uint16_t` inputs, bit-for-bit; covers every one of the 64 exponent rows, both `m__offset` values, and the inf/NaN rows `n == 31` / `n == 63` | [x] |
| C29 | `f11`, `s == 0` | early-return arm; `l` random incl. `±inf`, NaN, subnormal; `h` random (must be ignored) | [x] |
| C30 | `f11`, hue sector `0 <= h < 60` | random `h` in range × random `s != 0`, `l` | [x] |
| C31 | `f11`, hue sector `60 <= h < 120` | random `h` in range × random `s != 0`, `l` | [x] |
| C32 | `f11`, hue `120 <= h < 180` | the C's third arm is `h < 120 && h < 180`, so this range actually falls through to the **final `else`** (all three outputs `= m`). Verified against C rather than against the intent | [x] |
| C33 | `f11`, hue sector `180 <= h < 240` | random `h` in range × random `s != 0`, `l` | [x] |
| C34 | `f11`, hue sector `240 <= h < 300` | random `h` in range × random `s != 0`, `l` | [x] |
| C35 | `f11`, hue sector `300 <= h < 360` | random `h` in range × random `s != 0`, `l` | [x] |
| C36 | `f11`, `h` out of `[0,360)` / NaN | `h < 0`, `h >= 360`, `h = ±inf`, `h = NaN` → final `else`; also `h < 60` reached via **negative** `h`? (no: `h >= 0.0f` guards it) — verified | [x] |
| C37 | `f11`, `l`/`s` extremes | `l` ∈ {0, 0.5, 1, -1, 2, inf, NaN}, `s` ∈ {tiny subnormal, 1, -1, inf, NaN} crossed with the C30–C35 sectors — drives `fabsf`/`fmodf` and the `c`/`m`/`x` NaN paths | [x] |
| C38 | `f12`, `s == 0` | early-return arm; `v` random incl. specials | [x] |
| C39 | `f12`, `i == 0..=4` | `h` chosen so `(int)floorf(h/60)` lands on each of 0,1,2,3,4 (one row per `switch` arm), × random `s != 0`, `v` | [x] |
| C40 | `f12`, `switch default:` | `i` outside `0..=4`: `h >= 300`, `h` negative, `h` huge (`1e30`), `h = ±inf`, `h = NaN` (x86 `cvttss2si` → `INT_MIN`) | [x] |
| C41 | `f12`, sector boundaries | `h` exactly `0, 60, 120, 180, 240, 300, 360`, and one ULP either side of each (`f = h/60 - i` is exactly 0 or nearly 1) | [x] |
| C42 | `f13`, `max == r` | `r` strictly greatest, `delta != 0`; incl. the `h < 0` → `h += 360` correction (`g < b`) | [x] |
| C43 | `f13`, `max == g` | `g` strictly greatest → `h = 2 + (b-r)/delta` | [x] |
| C44 | `f13`, `max == b` | `b` strictly greatest → `h = 4 + (r-g)/delta` | [x] |
| C45 | `f13`, ties | `r == g > b`, `g == b > r`, `r == b > g` — the C's `==` comparisons pick the *first* matching branch, so ties are branch-order-sensitive | [x] |
| C46 | `f13`, `delta == 0` / `max == 0` | `r == g == b` (incl. all `0.0`, all `-0.0`, all `inf`, all `1.0`); and `max == 0` with negative channels | [x] |
| C47 | `f13`, negative / special channels | mixed-sign, `±inf`, subnormal, NaN inputs — drives the `delta` NaN path (E16) and `inf - inf` | [x] |
| C48 | `agglom`, full composition | all 33 arguments seeded-random over the full bit range (uniform random `u32`/`u64` bit patterns reinterpreted as floats, so NaN/inf/subnormal appear naturally); the `isnan` filters and `f32 → f64` widening are part of what is compared | [x] |
| C49 | `agglom`, per-sub-function sweeps | 13 sub-sweeps: hold 32 of the 33 arguments at a fixed "tame" baseline and sweep the arguments belonging to one sub-function through that sub-function's C-table configurations — isolates which composed stage diverges | [x] |
| C50 | `agglom`, all-zero / all-ones / extremes | every argument `0`; every integer argument `MAX`; every float argument `±inf`, NaN, `±0.0`, `FLT_MIN`, `FLT_MAX`; and the `f4` `{0,0}` degenerate state combined with each | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)` and
no `add_executable`. `translation/Cargo.toml` declares `crate-type = ["cdylib"]`
and has no `[[bin]]` and no `src/main.rs`. **Neither side builds a driver
binary**, so the "compare stdout byte-for-byte" gate is not applicable.

## Row → test mapping (all rows checked off; `./verify.sh` re-runs everything)

| rows | test function | file |
|------|---------------|------|
| C1 | `c1_c2v` | `tests/phase_b_c2.rs` |
| C2, C4, C5 | `c2_c4_c5_binary_vector_ops` | `tests/phase_b_c2.rs` |
| C3 | `c3_c2clampv` | `tests/phase_b_c2.rs` |
| C6, C9 | `c6_c9_circle_to_circle_direct_and_via_f2` | `tests/phase_b_c2.rs` |
| C7, C10, C11 | `c7_c10_c11_circle_to_aabb_direct_and_both_f2_orders` | `tests/phase_b_c2.rs` |
| C8, C12 | `c8_c12_aabb_to_aabb_direct_and_via_f2` | `tests/phase_b_c2.rs` |
| C13–C16 | `c13_c16_f3_all_sign_quadrants` | `tests/phase_b_scalar.rs` |
| C17 | `c17_f3_exhaustive_small_grid` | `tests/phase_b_scalar.rs` |
| C18 | `c18_f4_single_call_value_and_mutated_state` | `tests/phase_b_scalar.rs` |
| C19 | `c19_f4_thousand_call_chain` | `tests/phase_b_scalar.rs` |
| C20 | `c20_f5_bit_reversal` | `tests/phase_b_scalar.rs` |
| C21–C24 | `c21_c24_f7_all_four_flag_combinations` | `tests/phase_b_scalar.rs` |
| C25 | `c25_f9_non_degenerate_triangles` | `tests/phase_b_f9_f10.rs` |
| C26 | `c26_f9_degenerate_triangles` | `tests/phase_b_f9_f10.rs` |
| C27 | `c27_f9_special_floats` | `tests/phase_b_f9_f10.rs` |
| C28 | `c28_f10_exhaustive_all_65536_inputs` | `tests/phase_b_f9_f10.rs` |
| C29–C37 | `c29_c37_f11_all_sectors_and_early_return` | `tests/phase_b_hsv.rs` |
| C38–C41 | `c38_c41_f12_all_switch_arms_and_early_return` | `tests/phase_b_hsv.rs` |
| C42–C47 | `c42_c47_f13_max_channel_selection_and_degenerates` | `tests/phase_b_hsv.rs` |
| C48 | `c48_agglom_full_random_bit_space` | `tests/phase_b_agglom.rs` |
| C49 | `c49_agglom_per_subfunction_sweeps` | `tests/phase_b_agglom.rs` |
| C50 | `c50_agglom_all_zero_all_ones_and_extremes` | `tests/phase_b_agglom.rs` |

## Divergences found and fixed while checking these rows

All were x86 NaN-payload / operand-order bugs, found by comparing `to_bits()`
rather than float values, and diagnosed from `objdump -d` of the C `.so` (CMake
builds it at `-O0`, so GCC emits one SSE instruction per source operator and the
*first* source operand — the one whose NaN payload survives — is always the
operator's left-hand side).

1. `c2Dot` / `lm_dot2` — GCC emits the second product as `mulss(b.y, a.y)` and
   makes it the *first* source of the `addss`. The Rust had
   `addss(mulss(a.x,b.x), mulss(a.y,b.y))`, which returned the wrong NaN
   payload. Fixed to `addss(mulss(b.y, a.y), mulss(a.x, b.x))`.
2. `c2CircletoCircle` — `A.r + B.r` is emitted as `addss(B.r, A.r)`.
3. `f9` — five of the eight products in `invDenom`/`u`/`v` had swapped operands.
4. `f11` — four of the `x + m` / `c + m` stores in the later hue sectors had
   swapped operands (the original comments claimed "GCC swaps"; it does not).
5. The `addss`/`subss`/`mulss`/`divss` helpers were written as
   `if a.is_nan() {...} else if b.is_nan() {...} else { a op b }`. That is
   correct at `-O0` but **LLVM restructures it at `-O`**: `f9` returned
   `0xffc00000` instead of C's `0xffc58ed7` under `--release` while passing in
   debug. Rewritten as inline `asm!` so the instruction and operand order are
   opaque to the optimiser. Every row is now run in both profiles.
