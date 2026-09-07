# ERRORS.md — error/rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping for every early return,
every sentinel/guard, every `default:` label, every `assert`, and every
range/null check. Findings:

* `grep -n "assert"` → **0 hits**. The library has no assertions.
* `grep -n "RETURN_ERROR\|return -1\|return NULL"` → **0 hits**. There is no
  error enum, no `errno` use, and no out-parameter status code.
* `grep -n "malloc\|calloc\|free"` → **0 hits** in the library body (only
  `<stdlib.h>` is included), so there is no allocation-failure path.
* The library's *entire* rejection surface is therefore: the `default:` arms of
  the two-level `switch` in `f2` (line 91/92, 101/102, 105/106), the
  divide-by-zero guard in `f3` (line 111/112), and the degenerate-input early
  returns in `f11` (line 872), `f12` (line 919) and `f13` (line 984).
* `f11`/`f12`/`f13` dereference `dest`/`src` with **no** null check, so a null
  pointer is UB in C, not a rejection. It is listed below as a boundary row that
  is deliberately NOT exercised (see row B4).
* `f10` indexes `m__mantissa[(h & 0x3ff) + m__offset[h >> 10]]` with **no**
  bounds check; `h` is `uint16_t` so `h >> 10 <= 63` and the composed index is
  always in `0..2048`. There is no reachable rejection — every one of the 65536
  inputs is valid (row B5 exhaustively proves it).

## Rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `f2` | `typeA == C2_TYPE_CIRCLE (0)`, `typeB` is any value that is not 0 or 1 (e.g. 2, 3, 7, 255, `INT_MAX`, `0xFFFFFFFF`) → inner `switch` `default:` at line 91 | returns `0`; neither `A` nor `B` is dereferenced |
| E2 | `f2` | `typeA == C2_TYPE_AABB (1)`, `typeB` is any value that is not 0 or 1 → inner `switch` `default:` at line 101 | returns `0`; neither `A` nor `B` is dereferenced |
| E3 | `f2` | `typeA` is any value that is not 0 or 1 (out-of-range enum across the FFI boundary), `typeB` arbitrary (incl. valid 0/1) → outer `switch` `default:` at line 105 | returns `0`; neither `A` nor `B` is dereferenced |
| E4 | `f3` | `v2 == 0` (divide-by-zero guard, line 111) — for every `v1` incl. `0`, `INT_MIN`, `INT_MAX` | returns `0` (no trap, no SIGFPE) |
| E5 | `f3` | `v1 == INT_MIN`, `v2 == -1` — the one signed-division overflow pair; C reaches the `v1 == INT_MIN`/`v2 < 0`/`v2 != INT_MIN` arm which computes `-(v1-v2)` etc. and never executes `INT_MIN / -1` | returns `INT_MIN` (`-2147483648`), no trap |
| E6 | `f3` | `v1 == INT_MIN`, `v2 == INT_MIN` → the final `else` arm `q = 1, r = 0` | returns `1` |
| E7 | `f3` | `v1 == INT_MIN`, `v2 > 0` → `q = -((-(v1+v2))/v2) - 1` arm, incl. the `v2 == 1` sub-case where `-(v1+v2)` itself overflows | matches C exactly (wrapping); e.g. `f3(INT_MIN, 1) == INT_MIN` |
| E8 | `f3` | `v1 >= 0`, `v2 == INT_MIN` → `q = 0, r = v1` arm; `r >= 0` unless `v1 == 0`… (`v1>=0` so `r>=0`) | returns `0` |
| E9 | `f3` | `v1 < 0 && v1 != INT_MIN`, `v2 == INT_MIN` → `q = 1, r = v1 - q*v2` arm; `r = v1 - INT_MIN` wraps | returns `1` if `r >= 0`, else `1 + 1 = 2` (`v2 < 0`) |
| E10 | `f11` | `s == 0.0f` (incl. `-0.0f`, which compares equal) → early return at line 872 | writes `l` to all three of `dest[0..2]`; `h`/NaN-ness of `h` is irrelevant |
| E11 | `f11` | `h` is NaN, or `h` outside every `if` range (`h < 0`, `h >= 360`, or `120 <= h < 180` is *unreachable* because the third arm reads `h < 120 && h < 180`) → final `else` | writes `m` to all three of `dest[0..2]` |
| E12 | `f12` | `s == 0.0f` (incl. `-0.0f`) → early return at line 919 | writes `v` to all three of `dest[0..2]` |
| E13 | `f12` | `i = (int)floorf(h/60)` falls outside `0..=4` (negative `h`, huge `h`, or NaN `h` where the x86 `cvttss2si` yields `INT_MIN`) → `switch` `default:` at line 957 | writes `r=v, g=p, b=q` |
| E14 | `f13` | `delta == 0.0f` (all three channels equal, incl. all-`-0.0f`) → early return at line 984 | writes `0, 0, max` |
| E15 | `f13` | `max == 0.0f` with `delta != 0` (only possible when some channel is negative so that `max == 0` and `min < 0`) → same early return | writes `0, 0, 0.0f` (`v = max`) |
| E16 | `f13` | all inputs NaN / mixed NaN, so all of `min<g`, `min<b`, `max>g`, `max>b` are false and `delta` is NaN → `delta == 0` false, `max == 0` false, then `r == max` false, `g == max` false → `h = 4 + (r-g)/delta` | matches C's NaN payload exactly |
| E17 | `f4` | state `{0, 0}` — the xorshift128+ fixed point; the generator is degenerate and never leaves 0 | returns `0.0` forever (`(1023<<52) | 0` reinterpreted, minus 1.0) |
| E18 | `f7` | `blocksize`/`bitdepth` large enough that `blocksize * bitdepth * channels` overflows `uint32_t` (defined wrapping in C, e.g. `f7(0xFFFFFFFF, 3, 0xFFFFFFFF)`) | wrapped `uint32_t` result, identical to Rust `wrapping_mul` chain |
| E19 | `f7` | `channels == 0` (nonsensical but accepted: `channels * (channels != 2)` is `0`, all three products vanish) | returns `18 + 0 + (7/8) == 18` |
| E20 | `f9` | degenerate triangle: `dot00*dot11 - dot01*dot01 == 0` (e.g. `p1 == p2 == p3`) → `invDenom = 1.0f/0.0f = ±inf`, then `0 * inf` | returns NaN in both components; exact NaN bit pattern must match |
| E21 | `f9` | any input component is NaN or ±inf | propagated NaN/inf with the exact same bit pattern as C |
| E22 | `f5` | high 16 bits of `a` set (e.g. `0xFFFF0000`) — the masks only cover the low 16 bits, so the high half is silently **discarded** | returns only the bit-reversed low 16 bits (result always `<= 0xFFFF`) |
| E23 | `f10` | `h == 0` and `h == 0xFFFF` (the two index extremes) and every `h >> 10 == 31` / `== 63` (the `0x47800000` / `0xc7800000` inf-encoding exponent rows) | table lookup result; no rejection, but these are the only rows where the exponent table is not a plain `n << 23` |
| B1 | *boundary* | `f2` with `typeA`/`typeB` = `-1` reinterpreted (`0xFFFFFFFF`) — one step past the enum range in the negative direction | `0` (falls to outer `default:`) |
| B2 | *boundary* | `f3` with `v2 = 1` and `v2 = -1` for every extreme `v1` | matches C |
| B3 | *boundary* | `f7` with each of `blocksize`/`channels`/`bitdepth` at `0`, `1`, `2`, `3`, `31`, `32`, `33`, `0xFFFFFFFF` (the `channels == 2` and `bitdepth != 32` branch boundaries) | matches C |
| B4 | *boundary* | null `dest`/`src` for `f11`/`f12`/`f13` | **UB in C — not tested.** The C dereferences unconditionally; there is no rejection to compare against. Documented for completeness only. |
| B5 | *boundary* | `f10` over the **complete** `uint16_t` domain (all 65536 inputs) | exhaustive bit-for-bit table check; proves the unchecked index is always in range |

## Status — every row has a passing differential test

Each row's test constructs the exact condition, calls **both** `.so`s through
`libloading`, and asserts the same sentinel (return value / written bytes), not
merely that both "failed somehow". Run with `cargo test` and `cargo test
--release` (both profiles matter: LLVM restructures float code at `-O`).

| rows | test function | file | [x] |
|------|---------------|------|-----|
| E1, E2, E3, B1 | `e1_e2_e3_b1_f2_out_of_range_enums_return_zero` | `tests/phase_c_errors.rs` | [x] |
| E4 | `e4_f3_divide_by_zero_guard_returns_zero` | `tests/phase_c_errors.rs` | [x] |
| E5, E6, E7, E8, E9 | `e5_e9_f3_int_min_overflow_arms` | `tests/phase_c_errors.rs` | [x] |
| E10, E11 | `e10_e11_f11_degenerate_and_out_of_range_hue` | `tests/phase_c_errors.rs` | [x] |
| E12, E13 | `e12_e13_f12_degenerate_and_switch_default` | `tests/phase_c_errors.rs` | [x] |
| E14, E15, E16 | `e14_e16_f13_degenerate_paths` | `tests/phase_c_errors.rs` | [x] |
| E17 | `e17_f4_degenerate_zero_state_is_a_fixed_point` | `tests/phase_c_errors.rs` | [x] |
| E18, E19 | `e18_e19_f7_overflow_and_zero_channels` | `tests/phase_c_errors.rs` | [x] |
| E20, E21 | `e20_e21_f9_degenerate_and_special_inputs` | `tests/phase_c_errors.rs` | [x] |
| E22 | `e22_f5_discards_the_high_half` | `tests/phase_c_errors.rs` | [x] |
| E23, B5 | `e23_b5_f10_full_uint16_domain_and_exponent_extremes` | `tests/phase_c_errors.rs` | [x] |
| B2 | `b2_f3_unit_divisors_against_every_extreme` | `tests/phase_c_errors.rs` | [x] |
| B3 | `b3_f7_every_branch_boundary` | `tests/phase_c_errors.rs` | [x] |
| B4 | *not applicable* — null `dest`/`src` is UB in the C (no null check), so there is no rejection to compare | — | n/a |
| generic | `generic_boundaries_zero_and_oversized_and_one_past_range` — null pointers on every non-dereferencing `f2` path, aliased `f4` call sequences, in-place `dest == src` calls for `f11`/`f12`/`f13`, `f10(0xFFFF)` sign-extension | `tests/phase_c_errors.rs` | [x] |

### Out-of-range enum coverage (the class happy-path tests miss)

`f2`'s `C2_TYPE` parameters are exercised with `2, 3, 7, 255, 256, INT_MAX,
0x80000000, 0xFFFFFFFF (-1), 0xFFFFFFFE, 0x100`, in **both** parameter
positions, crossed with valid and invalid values in the other position, plus an
exhaustive sweep of `0..4096`. The C compares against literal `0` and `1`
(`cmpl`), so everything else falls to `default: return 0`; the Rust `match`
does the same. Both return `0` and neither dereferences the pointers, which is
also verified by passing `NULL` for both on every rejecting combination.

### Notes on what the C does NOT reject

* `f3` never traps: the `INT_MIN / -1` and `INT_MIN % -1` overflow pairs are all
  routed around by the explicit `v2 != (-0x7fffffff - 1)` guards, so the Rust
  must use wrapping ops (it does) and must never panic.
* `f5` silently discards the high 16 bits rather than rejecting them.
* `f7` accepts `channels == 0` and wraps on `uint32_t` overflow.
* `f10` has no bounds check but the index is provably in range for all 65536
  inputs (proved exhaustively, not argued).
