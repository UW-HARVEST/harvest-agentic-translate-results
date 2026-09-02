# CONFIGS.md — Configuration / valid-input surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axis enumeration (from the C source)

This library has **no runtime option struct, no init call, no flags, no global
state, and no `#ifdef`** (verified by grep). The "configuration" of a call is
therefore entirely (a) which entry point, and (b) the shape/value class of the
scalar arguments that the code branches on. Those axes are:

| axis | values the C distinguishes | source |
|---|---|---|
| `A1` entry point | `safe_double_to_int`, `process_array_reverse`, `switch_fallthrough_calculator`, `allocate_and_compute`, `foreach_sum`, `fallcalc` | the 6 external-linkage definitions |
| `A2` `double` value class | NaN / `+inf` / `-inf` / `>= 2^31-1` / `<= -2^31` / normal fractional / integral / `0.0` / `-0.0` / subnormal | `isnan`, `isinf`, `>=`, `<=` guards in `safe_double_to_int` |
| `A3` truncation sign | `d > 0` (truncates down in magnitude) vs `d < 0` (C cast truncates toward zero, **not** floor) | `(int)d` |
| `A4` element count | `0`, `1`, `2`, many, negative | `i < count` / `keep && idx < size` loop guards |
| `A5` traversal direction | forward (`foreach_sum`, via `FOREACH`) vs backward (`process_array_reverse`, `ptr--`) | the two loop bodies |
| `A6` `operation` arm | `0`, `1`, `2` (fall-through chain ending in `& 0777`), `3`, `4` (chain ending in `+ 0100`), `default` | the `switch` labels |
| `A7` fall-through depth | arm `0` runs 3 statements, `1` runs 2, `2` runs 1, `3` runs 2, `4` runs 1 | absence of `break` after cases 0,1,3 |
| `A8` `value` magnitude | small, near `INT_MAX/8`, near `INT_MAX/3`, `INT_MAX`, `INT_MIN`, negative | `*= 8`, `*= 3`, `+= 128`, `+= 64` overflow |
| `A9` `size` for allocation | `0`, `1`, `2`, small, large-but-allocatable, negative | `malloc(size * sizeof(DataPoint))` + loop guards |
| `A10` `multiplier` | `0.0`, `-0.0`, `1.5`, negative, tiny, huge, NaN, `±inf` | `(double)i * multiplier` |
| `A11` `fallcalc` `param3 > 0200` flag | true → `result \|= 0200`; false → no-op | `if (param3 > OCTAL_FLAG)` |
| `A12` `fallcalc` `param3 % 5` residue | `0, 1, 2, 3, 4` and `-1, -2, -3, -4` (C `%` truncates toward zero) | `switch_fallthrough_calculator(param2, param3 % 5)` |
| `A13` `fallcalc` `param4 % 10 + 1` size | `1..10` and `0, -1 … -8` | `allocate_and_compute(param4 % 10 + 1, 1.5)` |
| `A14` final mask | every result is `& 0777`, so only the low 9 bits are observable from `fallcalc` | `result &= OCTAL_MASK_1` |
| `A15` allocator identity | Rust must call libc `malloc`/`free`, not Rust's allocator, or `size == 0` and failure behaviour diverges | `nm -D` shows `U malloc`/`U free` on both |
| `A16` build features | **none** — `Cargo.toml` has no `[features]`; default build is the only combination | `Cargo.toml` |

## Configuration rows

One row per meaningful combination the C treats differently. Each row is
exercised with **many randomized inputs** (fixed seed, SplitMix64 PRNG) plus the
listed boundary values, comparing C `.so` vs Rust `.so` byte-for-byte.

### Low-level entry points first (called directly, not only through `fallcalc`)

| # | entry point(s) | configuration (options set + input shape) | ok |
|---|----------------|--------------------------------------------|-----|
| C1 | `safe_double_to_int` | A2 special classes: NaN (quiet/negative/signalling), `±inf`, `0.0`, `-0.0`, subnormal `5e-324`, `±DBL_MAX`, `±DBL_MIN` | [x] |
| C2 | `safe_double_to_int` | A2 boundary: exactly `2147483647.0`, `2147483646.0`, `2147483647.5`, `-2147483648.0`, `-2147483647.0`, `-2147483648.5`, and `nextafter` neighbours of each | [x] |
| C3 | `safe_double_to_int` | A2×A3 in-range positive: random uniform `[0, 2^31)` incl. fractional parts — checks truncation toward zero | [x] |
| C4 | `safe_double_to_int` | A2×A3 in-range negative: random uniform `(-2^31, 0]` incl. fractional parts — checks truncation toward zero, not floor | [x] |
| C5 | `safe_double_to_int` | A2 random raw 64-bit bit patterns reinterpreted as `double` (covers all classes incl. NaN payloads, huge exponents) | [x] |
| C6 | `process_array_reverse` | A4=`1`, backward walk of a 1-element buffer | [x] |
| C7 | `process_array_reverse` | A4=`2`, buffer end pointer, random values | [x] |
| C8 | `process_array_reverse` | A4=many (`3..64`), random `int` values incl. `INT_MIN`/`INT_MAX` — exercises A5 backward direction and wrapping accumulation | [x] |
| C9 | `process_array_reverse` | A4=many, partial walk: `count` smaller than the buffer, `end` pointing into the middle | [x] |
| C10 | `process_array_reverse` | A8 overflow: all elements near `INT_MAX` so `sum` wraps | [x] |
| C11 | `foreach_sum` | A4=`1`, single element | [x] |
| C12 | `foreach_sum` | A4=`2` | [x] |
| C13 | `foreach_sum` | A4=many (`3..64`), random values — exercises A5 forward direction and the `FOREACH` double-loop expansion (each element visited exactly once) | [x] |
| C14 | `foreach_sum` | A8 overflow: elements near `INT_MIN` so `total` wraps | [x] |
| C15 | `foreach_sum` vs `process_array_reverse` | same buffer both directions: sums must be equal (order-independence cross-check that `FOREACH` visits every element exactly once) | [x] |
| C16 | `switch_fallthrough_calculator` | A6=`0` × A7 depth 3 (`*8`, `+128`, `&511`) × A8 small values | [x] |
| C17 | `switch_fallthrough_calculator` | A6=`0` × A8 overflow: `value` near `INT_MAX`/`INT_MIN` so `*8` wraps before the mask | [x] |
| C18 | `switch_fallthrough_calculator` | A6=`1` × A7 depth 2 (`+128`, `&511`) × A8 random, incl. `INT_MAX` (so `+128` wraps) | [x] |
| C19 | `switch_fallthrough_calculator` | A6=`2` × A7 depth 1 (`&511` only) × A8 random incl. negatives — checks C's `&` on negative two's-complement | [x] |
| C20 | `switch_fallthrough_calculator` | A6=`3` × A7 depth 2 (`*3`, `+64`, **no mask**) × A8 random — result is unmasked, so the full 32-bit value is observable | [x] |
| C21 | `switch_fallthrough_calculator` | A6=`3` × A8 overflow: `value` near `INT_MAX/3` and `INT_MIN/3` so `*3` wraps | [x] |
| C22 | `switch_fallthrough_calculator` | A6=`4` × A7 depth 1 (`+64`, no mask) × A8 random incl. `INT_MAX` (so `+64` wraps) | [x] |
| C23 | `switch_fallthrough_calculator` | A6=all arms × A8 full-random `value` (cross-product sweep, `operation` in `-8..=8`) | [x] |
| C24 | `allocate_and_compute` | A9=`1` × A10=`1.5` — single point, `sum` = `0*0` = `0` | [x] |
| C25 | `allocate_and_compute` | A9=`2` × A10=`1.5` | [x] |
| C26 | `allocate_and_compute` | A9=small (`1..=10`, the range `fallcalc` can produce) × A10=`1.5` | [x] |
| C27 | `allocate_and_compute` | A9=small × A10=`0.0` and `-0.0` — `sum` is `0.0` / `-0.0`, both convert to `0` | [x] |
| C28 | `allocate_and_compute` | A9=small × A10 negative random — `sum` negative, exercises A3 negative truncation | [x] |
| C29 | `allocate_and_compute` | A9=small × A10 random uniform, non-integral — exercises FP accumulation order (must match term by term) | [x] |
| C30 | `allocate_and_compute` | A9=small × A10 tiny (`1e-300`, subnormal) — `sum` underflows toward `0` | [x] |
| C31 | `allocate_and_compute` | A9=small × A10 large (`1e9`, `1e18`) — `sum` exceeds `INT_MAX`, hits the E6 saturation from inside | [x] |
| C32 | `allocate_and_compute` | A9=larger (`64`, `1000`, `65536`) × A10 random — exercises the `i * 8` `value` field growing and longer FP accumulation chains | [x] |
| C33 | `allocate_and_compute` | A9 × A10 full-random cross-product sweep | [x] |
| C34 | `allocate_and_compute` | A15: repeated alloc/free cycles interleaved between the two `.so`s, confirming both use the same libc heap and `size == 0` behaves identically | [x] |

### Composed top-level entry point

| # | entry point(s) | configuration (options set + input shape) | ok |
|---|----------------|--------------------------------------------|-----|
| C35 | `fallcalc` | A11=false (`param3 <= 128`) × A12 residue `0` × A13 size `1..10` | [x] |
| C36 | `fallcalc` | A11=false × A12 residue `1` | [x] |
| C37 | `fallcalc` | A11=false × A12 residue `2` | [x] |
| C38 | `fallcalc` | A11=false × A12 residue `3` | [x] |
| C39 | `fallcalc` | A11=false × A12 residue `4` | [x] |
| C40 | `fallcalc` | A11=true (`param3 > 128`, so `result \|= 0200`) × A12 residues `0..4` — `param3 > 128` forces `param3 % 5` non-negative | [x] |
| C41 | `fallcalc` | A12 negative residues `-1, -2, -3, -4` (`param3` negative) → `default` arm, `switch_result = 0`; A11 necessarily false | [x] |
| C42 | `fallcalc` | A13 = each of `1..=10` (i.e. `param4 % 10 ∈ 0..=9`) held fixed while other params randomize | [x] |
| C43 | `fallcalc` | A13 <= 0 (`param4 % 10 ∈ -9..=-1`), so the nested `allocate_and_compute` returns `-1` or `0` | [x] |
| C44 | `fallcalc` | A8/A14 overflow: `param1` near `INT_MAX`/`INT_MIN` so `param1 * 0100` wraps, and `param1 * 3.7` saturates | [x] |
| C45 | `fallcalc` | A2/A24: `param1`,`param2`,`param3` chosen so `floating_calc` lands just inside / just outside `±INT_MAX` | [x] |
| C46 | `fallcalc` | exhaustive small cube: `param1,param2,param3,param4 ∈ -6..=6` (13^4 = 28 561 calls) — covers every A11×A12×A13 interaction at small magnitudes | [x] |
| C47 | `fallcalc` | full-random 32-bit cross-product sweep of all four params (100 000 iterations, fixed seed) | [x] |
| C48 | `fallcalc` | boundary sweep: all 4^4 combinations drawn from `{INT_MIN, -1, 0, 1, 128, 129, INT_MAX, INT_MIN+1, -128, 5, 10, -10}` | [x] |
| C49 | consistency | `fallcalc` recomputed from its documented pieces via the low-level exports of the **same** `.so`, confirming the composed pipeline in Rust wires the same sub-results as C (catches wiring bugs invisible to per-function tests) | [x] |

## Feature combinations (A16)

`Cargo.toml` has no `[features]` table, so the complete set of feature
combinations is `{ default }` = `{ --no-default-features }` = one build.
`./check_features.sh` enumerates them from `Cargo.toml` and runs
`cargo check` + the full test suite for each; it finds exactly one.
