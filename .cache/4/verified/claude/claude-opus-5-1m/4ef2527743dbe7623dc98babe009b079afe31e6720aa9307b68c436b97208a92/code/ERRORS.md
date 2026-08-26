# ERRORS.md — ERROR-SURFACE TABLE (Phase C)

Every distinct way `c_src/src/lib.c` rejects / short-circuits / degenerates on
input, derived mechanically from the source. The library has **no** error enum,
no `assert`, no `malloc`, no `RETURN_ERROR` macro and no explicit `NULL` check —
its whole rejection surface consists of `switch` `default:` arms, guard `if`s
that return a sentinel/early value, and the branches that exist purely to avoid
signed-overflow UB. All of those are enumerated below.

Test file: `tests/errors.rs`. Every row has a dedicated differential test that
calls **both** `.so`s through `libloading` and compares the raw bit patterns.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ok |
|---|----------|------------------------------------------|-------------------|------|----|
| E01 | `f2` (`lib.c:105-106`) | `typeA` is neither `C2_TYPE_CIRCLE`(0) nor `C2_TYPE_AABB`(1) — outer `switch` `default:` (e.g. `typeA = 2, 3, -1, INT_MIN, INT_MAX`) | `return 0` (A/B never dereferenced) | `e01_f2_bad_typeA` | [x] |
| E02 | `f2` (`lib.c:91-92`) | `typeA == C2_TYPE_CIRCLE` and `typeB` out of range — inner `switch` `default:` | `return 0` | `e02_f2_circle_bad_typeB` | [x] |
| E03 | `f2` (`lib.c:101-102`) | `typeA == C2_TYPE_AABB` and `typeB` out of range — inner `switch` `default:` | `return 0` | `e03_f2_aabb_bad_typeB` | [x] |
| E04 | `f2` (`lib.c:105-106`) | `typeA` out of range **and** `A == NULL, B == NULL` (out-of-range enum makes the null pointers unreachable) | `return 0`, no crash | `e04_f2_bad_type_null_ptrs` | [x] |
| E05 | `f3` (`lib.c:111-113`) | `v2 == 0` (division-by-zero guard) — for **all** `v1`, incl. `INT_MIN`/`INT_MAX` | `return 0` | `e05_f3_zero_divisor` | [x] |
| E06 | `f3` (`lib.c:118, 121`) | `v1 >= 0` and `v2 == INT_MIN` (`-0x7fffffff - 1`): `-v2` would overflow → special arm `q = 0, r = v1` | `q=0,r=v1`; final `r>=0 ? q : q+(v2>0?-1:1)` ⇒ `0` for `v1 >= 0` | `e06_f3_v2_intmin` | [x] |
| E07 | `f3` (`lib.c:125, 128`) | `v1 < 0 && v1 != INT_MIN` and `v2 == INT_MIN`: special arm `q = 1, r = v1 - q*v2` | `r = v1 - INT_MIN >= 0` ⇒ `return 1` | `e07_f3_v1neg_v2_intmin` | [x] |
| E08 | `f3` (`lib.c:122, 129-130`) | `v1 == INT_MIN` and `v2 >= 1`: `-v1` would overflow → arm using `-(v1+v2)` | `q = -((-(v1+v2))/v2) - 1`, then sign fixup | `e08_f3_v1_intmin_v2_pos` | [x] |
| E09 | `f3` (`lib.c:131-132`) | `v1 == INT_MIN` and `v2 < 0 && v2 != INT_MIN`: arm using `-(v1-v2)` | `q = ((-(v1-v2))/(-v2)) + 1`, then sign fixup | `e09_f3_v1_intmin_v2_neg` | [x] |
| E10 | `f3` (`lib.c:133-134`) | `v1 == INT_MIN` **and** `v2 == INT_MIN`: last-resort arm `q = 1, r = 0` | `return 1` | `e10_f3_both_intmin` | [x] |
| E11 | `f3` (`lib.c:135-138`) | any arm leaving `r < 0` (non-exact division with mixed signs) → floor correction | `return q + (v2 > 0 ? -1 : 1)` | `e11_f3_negative_remainder` | [x] |
| E12 | `f4` (`lib.c:145-154`) | degenerate RNG state `{0, 0}` (xorshift fixed point) | `cn_rnd_next` returns `0` forever ⇒ `f4` returns `+0.0`, state stays `{0,0}` | `e12_f4_zero_state` | [x] |
| E13 | `f7` (`lib.c:451-458`) | `blocksize`/`bitdepth`/`channels` large enough to overflow `tflac_u32` in the numerator (e.g. `0xFFFFFFFF`) — no range check at all | wrap-around `uint32_t` arithmetic, `+7` then `/8` on the wrapped value | `e13_f7_overflow` | [x] |
| E14 | `f7` (`lib.c:451-458`) | `channels == 0` and/or `bitdepth == 0` and/or `blocksize == 0` (no validation) | `18 + channels + (0+7)/8` = `18 + channels` when the numerator is 0 | `e14_f7_zero_args` | [x] |
| E15 | `f9` (`lib.c:486`) | degenerate triangle: `dot00*dot11 - dot01*dot01 == 0` ⇒ `invDenom = 1.0f/0.0f` | `±inf` invDenom ⇒ `±inf`/`NaN` components (no rejection) | `e15_f9_degenerate` | [x] |
| E16 | `f9` (`lib.c:486`) | `p1 == p2 == p3` (all three vertices identical) ⇒ `0.0f * 0.0f - 0.0f == 0.0f` | `invDenom = +inf`, `u = 0*inf = NaN`, `v = NaN` | `e16_f9_all_points_equal` | [x] |
| E17 | `f10` (`lib.c:857-865`) | `h` with `h >> 10 == 31` or `63` (`m__exponent[31] = 0x47800000`, `[63] = 0xc7800000`) — the half-float inf/NaN rows; **no** bounds check on the table index | `m__mantissa[(h&0x3ff)+0x400] + exponent` ⇒ inf/NaN encodings | `e17_f10_inf_nan_rows` | [x] |
| E18 | `f10` (`lib.c:857-865`) | **exhaustive**: all 65 536 `uint16_t` values (index is `(h&0x3ff)+m__offset[h>>10]` ≤ 2047, never out of bounds) | table lookup + `uint32_t` wrapping add, reinterpreted as `float` | `e18_f10_exhaustive` | [x] |
| E19 | `f11` (`lib.c:872-877`) | `s == 0.0f` (also `-0.0f`) → early return | `dest[0..3] = l, l, l` (returns before touching `h`) | `e19_f11_s_zero` | [x] |
| E20 | `f11` (`lib.c:905-909`) | `h` matching no band — the `else` arm. Reachable for `h >= 360.0f`, `h == NaN`, and `+inf` (note the `h < 120.0f && h < 180.0f` typo makes negative `h` hit band 3, **not** the else arm) | `dest[0..3] = m, m, m` | `e20_f11_else_band` | [x] |
| E21 | `f11` (`lib.c:889`) | negative `h` (`h < 0`) with `s != 0` — falls through bands 1 & 2 into the buggy `h < 120.0f && h < 180.0f` arm | `dest = {m, c+m, x+m}` (band-3 assignment, *not* the else arm) | `e21_f11_negative_h_hits_band3` | [x] |
| E22 | `f11` (`lib.c:880`) | `h` non-finite / huge so `h/60.0f` is `±inf` or `NaN` ⇒ `fmodf(±inf, 2)` = `NaN` | `x = NaN`, propagated into whichever band is selected | `e22_f11_nonfinite_h` | [x] |
| E23 | `f12` (`lib.c:919-924`) | `s == 0.0f` (also `-0.0f`) → early return | `dest[0..3] = v, v, v` | `e23_f12_s_zero` | [x] |
| E24 | `f12` (`lib.c:957-961`) | `i = (int)floorf(h/60.0f)` outside `0..=4` — `switch` `default:`. gcc emits an **unsigned** `cmpl $4 / ja`, so negative `i` also lands here | `r=v, g=p, b=q` | `e24_f12_default_sector` | [x] |
| E25 | `f12` (`lib.c:926`) | `h/60.0f` is `NaN` or out of `int` range (`|h/60| >= 2^31`) ⇒ `(int)floorf(...)` is UB; x86-64 `cvttss2si` yields the integer-indefinite value `INT_MIN` | `i = INT_MIN` ⇒ `default:` arm, `f = h - (float)INT_MIN` | `e25_f12_int_conversion_indefinite` | [x] |
| E26 | `f13` (`lib.c:984-989`) | `delta == 0.0f` (r == g == b) → early return | `dest = {0.0f, 0.0f, max}` | `e26_f13_delta_zero` | [x] |
| E27 | `f13` (`lib.c:984-989`) | `max == 0.0f` (also all-negative input where max is `-0.0f`, since `-0.0f == 0.0f`) → early return | `dest = {0.0f, 0.0f, max}` | `e27_f13_max_zero` | [x] |
| E28 | `f13` (`lib.c:991-996`) | `NaN` in `src`: every `comiss` compares false, so `min`/`max` collapse to the *last* operand; `r == max` and `g == max` are both false ⇒ the final `else` arm | `h = 4 + (r-g)/delta`, then `*60`, then the `h < 0` fixup is skipped for NaN | `e28_f13_nan_input` | [x] |
| E29 | `f13` (`lib.c:998-999`) | `h < 0` after `h *= 60` (only reachable via `r == max` with `g < b`) | `h += 360.0f` | `e29_f13_negative_hue_wrap` | [x] |
| E30 | `agglom` (`lib.c:1033-1101`) | any component result being `NaN` — the 12 `!isnan(...)` guards **skip** the accumulation | that term contributes nothing to `ret` | `e30_agglom_nan_terms_skipped` | [x] |
| E31 | generic FFI boundary | out-of-range `C2_TYPE` enum values passed as `int` across FFI: `-2147483648 … 2147483647` sampled + all of `-4..=5` for both `typeA` and `typeB` | identical `int` return from both `.so`s (`0` unless both are in `{0,1}`) | `e31_f2_enum_cross_product` | [x] |
| E32 | generic FFI boundary | `NULL` `src`/`dest` passed to `f11`/`f12`/`f13`, and `NULL` `rnd` to `f4` — the C never null-checks, so both libraries must fault identically | both terminate on `SIGSEGV` (compared in child processes) | `e32_null_pointer_parity` | [x] |
| E33 | generic FFI boundary | `f5` with the high 16 bits set (`a & 0xAAAA` etc. only touch bits 0..15, so bits 16..31 are silently discarded) | `f5(x) == f5(x & 0xFFFF)`, high bits dropped | `e33_f5_high_bits_dropped` | [x] |
| E34 | generic FFI boundary | one step past documented ranges: `f10(0)`, `f10(0xFFFF)`, `f3(INT_MIN, ±1)`, `f3(INT_MAX, ±1)`, `f7(u32::MAX, u32::MAX, u32::MAX)` | identical bits from both `.so`s | `e34_boundary_values` | [x] |

## Notes

* **All 34 rows have a passing differential test** (`cargo test --test errors`,
  34 passed / 1 ignored — the ignored one is the child-process helper used by
  E32). Verified in both the `dev` and the `release` profile.
* **E32 required a build-configuration fix, not a code fix.** Rust's
  `debug-assertions` insert a null-pointer-dereference check, which turned
  `f4(NULL)` into a panic → `SIGABRT` (signal 6) while the C produced
  `SIGSEGV` (signal 11). Since this crate's contract is to reproduce the C
  *including its UB*, `debug-assertions`/`overflow-checks` are now disabled for
  `[profile.dev]`/`[profile.test]`, matching `[profile.release]`, which already
  faulted identically to the C. Verified directly:
  `f4(NULL)` → signal 11 from the C `.so`, the release Rust `.so` **and** the
  dev Rust `.so`.
* **E11 corrected a wrong expectation in the test, not in the library.** The C's
  `f3` is *not* exactly mathematical floor division: for two negative operands
  with a non-zero remainder its correction over-shoots, e.g.
  `f3(-50, -49) == 2` while `floor(-50 / -49) == 1`. Per the "C is always
  correct" rule the test now models the C arm-by-arm instead of comparing
  against `floor`.
* **Rows with no counterpart in the C.** There is no error enum, no `assert`,
  no allocation and no explicit `NULL` check anywhere in `c_src/src/lib.c`, so
  there are no `RETURN_ERROR`-style codes to compare — the rejection surface is
  exactly the `switch default:` arms, the guard `if`s and the
  signed-overflow-avoidance arms listed above.
* **Dead guard.** `agglom`'s `!isnan(f4_r)` check can never be false: `f4`
  builds `(1023 << 52) | mantissa` and subtracts `1.0`, so its result is always
  in `[0, 1)`. Removing it from the Rust is an equivalent program (confirmed by
  the mutation audit in `CONFIGS.md`); it is kept for fidelity to the C.
