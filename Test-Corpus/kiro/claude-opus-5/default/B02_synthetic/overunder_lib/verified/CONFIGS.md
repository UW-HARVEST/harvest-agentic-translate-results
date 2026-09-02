# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c`. This library exposes **no runtime
options, no flags, no modes, no `#ifdef`s and no init/config struct** — grep
finds 0 option fields and 0 conditional-compilation branches. Its
configuration surface is therefore entirely the *input shape* axes that the
code actually branches on, crossed with the full set of entry points.

## Axes the C code actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A1 `safe_double_to_int` class of `d` | `> INT_MAX`, `< INT_MIN`, NaN, in-range | lines 40–47 |
| A2 truncation direction of `(int)d` | positive fraction, negative fraction, exact integer, ±0.0 | line 47 (`(int)d` truncates toward zero) |
| A3 special doubles | ±inf, ±0.0, subnormal, `DBL_MAX`/`-DBL_MAX`, NaN payloads | lines 40–44 |
| A4 `process_with_fallthrough` `code` | `5`, `4`, `3`, `2`, `1`, `0`, anything else | lines 54–71 (fall-through chain) |
| A5 `base_value` magnitude | small, near `INT_MAX`, near `INT_MIN` | line 52 + unchecked `+=` |
| A6 `handle_pointer_operations` `value` | small, magnitudes where `value*2` overflows `int` | line 84 (`value * 2`) |
| A7 `copy_data_block` byte content | all-zero, all-`0xFF`, random incl. padding bytes 4–7 and 36–39, NaN/inf in `value`, non-NUL-terminated `label` | line 78 (`memcpy` of `sizeof(DataBlock)`) |
| A8 `overunder` sign of `a` | `a >= 0` ⇒ `a % 6` in `0..5` (real cases); `a < 0` ⇒ `a % 6` in `-5..0` (`default:`) | line 115 |
| A9 `overunder` `a % 6` residue | each of `0,1,2,3,4,5` selects a different fall-through depth | line 115 → A4 |
| A10 `overunder` `d*d + a*a` | no overflow; overflow to positive; overflow to **negative** ⇒ `sqrt(NaN)` | line 108 |
| A11 `overunder` `b` magnitude | drives `b * 2.7` past `INT_MAX` (row A1) and `base_value` wrap (A5) | lines 106, 115 |
| A12 `overunder` `c` magnitude | drives `c / 3.3` and `handle_pointer_operations(c)` overflow | lines 107, 128 |
| A13 `overunder` `a + b` | array slot 4, wraps on overflow | line 144 |

Entry points (**all five exports**, lowest-level first, not just the
convenience wrapper `overunder`):
`safe_double_to_int`, `process_with_fallthrough`, `copy_data_block`,
`handle_pointer_operations`, `overunder`.

## Configuration rows

Every row is exercised against **both** `.so`s with many randomized inputs
(fixed seed `0x5EED_1234`, deterministic SplitMix64) plus the hand-picked
boundary values named in the row. `[x]` = passing.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `safe_double_to_int` | in-range positive, random uniform in `[0, INT_MAX]`, fractional (A1 in-range + A2 positive) | [x] |
| 2 | `safe_double_to_int` | in-range negative, random uniform in `[INT_MIN, 0]`, fractional (A2 negative — truncation toward zero) | [x] |
| 3 | `safe_double_to_int` | exact integers and `±0.0`, `-0.0` (A2 exact / signed zero) | [x] |
| 4 | `safe_double_to_int` | magnitudes just below/at/above both guards: `±2147483646.5`, `±2147483647.0`, `nextafter` of each (A1 boundary) | [x] |
| 5 | `safe_double_to_int` | subnormals, `DBL_MIN`, `DBL_MAX`, `±inf`, quiet/signalling NaN bit patterns (A3) | [x] |
| 6 | `safe_double_to_int` | fully random 64-bit bit patterns reinterpreted as `double` (A1×A2×A3 cross-product, unbiased) | [x] |
| 7 | `process_with_fallthrough` | `code = 5` (deepest fall-through: +50+40+30+20+10) × random `base_value` (A4×A5) | [x] |
| 8 | `process_with_fallthrough` | `code = 4` × random `base_value` (A4×A5) | [x] |
| 9 | `process_with_fallthrough` | `code = 3` × random `base_value` (A4×A5) | [x] |
| 10 | `process_with_fallthrough` | `code = 2` × random `base_value` (A4×A5) | [x] |
| 11 | `process_with_fallthrough` | `code = 1` (no fall-through, `break`) × random `base_value` (A4×A5) | [x] |
| 12 | `process_with_fallthrough` | `code = 0` (value-discarding arm) × random `base_value` incl. `INT_MAX`/`INT_MIN` (A4×A5) | [x] |
| 13 | `process_with_fallthrough` | `code` in `1..5` × `base_value` near `INT_MAX`/`INT_MIN` (A4×A5 wrap interaction) | [x] |
| 14 | `process_with_fallthrough` | fully random `code` × fully random `base_value` (A4×A5 unpruned cross-product) | [x] |
| 15 | `copy_data_block` | all-zero source; verifies all `sizeof(DataBlock)` bytes incl. padding (A7) | [x] |
| 16 | `copy_data_block` | all-`0xFF` source, `label` with no NUL terminator (A7) | [x] |
| 17 | `copy_data_block` | random 40-byte source incl. padding bytes 4–7 / 36–39, `value` = NaN / ±inf / subnormal (A7) | [x] |
| 18 | `copy_data_block` | pre-poisoned destination, to prove every byte is overwritten and none beyond `sizeof(DataBlock)` is touched (A7) | [x] |
| 19 | `handle_pointer_operations` | small `value` (no overflow) — random in `[-1000, 1000]` (A6) | [x] |
| 20 | `handle_pointer_operations` | `value` where `value*2` overflows `int`: near `INT_MAX`/`INT_MIN`, plus fully random `i32` (A6) | [x] |
| 21 | `overunder` | `a >= 0`, small `a,b,c,d`, `a % 6 == 0` — switch `case 0` path (A8×A9 res 0) | [x] |
| 22 | `overunder` | `a >= 0`, `a % 6 == 1` — `case 1` (A9 res 1) × random `b,c,d` | [x] |
| 23 | `overunder` | `a >= 0`, `a % 6 == 2` — `case 2` fall-through (A9 res 2) × random `b,c,d` | [x] |
| 24 | `overunder` | `a >= 0`, `a % 6 == 3` — `case 3` fall-through (A9 res 3) × random `b,c,d` | [x] |
| 25 | `overunder` | `a >= 0`, `a % 6 == 4` — `case 4` fall-through (A9 res 4) × random `b,c,d` | [x] |
| 26 | `overunder` | `a >= 0`, `a % 6 == 5` — `case 5` deepest fall-through (A9 res 5) × random `b,c,d` | [x] |
| 27 | `overunder` | `a < 0` ⇒ negative residue ⇒ `default:` arm, `switch_result == -1` (A8) × random `b,c,d` | [x] |
| 28 | `overunder` | `a,d` small so `d*d + a*a` does **not** overflow ⇒ real `sqrt` (A10 no-overflow) | [x] |
| 29 | `overunder` | `a,d` large so `d*d + a*a` overflows to a **positive** `int` ⇒ `sqrt` of a wrapped positive (A10) | [x] |
| 30 | `overunder` | `a,d` large so `d*d + a*a` overflows to a **negative** `int` ⇒ `sqrt(neg) = NaN` ⇒ `conv4 == 0` (A10) | [x] |
| 31 | `overunder` | `b` large enough that `b * 2.7 > INT_MAX` ⇒ `conv2` saturates (A11 × A1) | [x] |
| 32 | `overunder` | `b` negative-large so `b * 2.7 < INT_MIN` ⇒ `conv2` saturates low (A11 × A1) | [x] |
| 33 | `overunder` | `c` near `INT_MAX`/`INT_MIN` ⇒ `handle_pointer_operations(c)` overflow inside the pipeline (A12 × A6) | [x] |
| 34 | `overunder` | `a + b` overflows (array slot 4 wraps) (A13) | [x] |
| 35 | `overunder` | all four args at the extremes `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` — full 7⁴ corner grid (A8–A13 boundary cross-product) | [x] |
| 36 | `overunder` | fully random `(a,b,c,d)` i32 quadruples — unpruned cross-product of every axis, many iterations | [x] |
| 37 | `overunder` | **stdout byte-comparison**: the 8 `printf` lines (incl. `%.2f` of `temp1` and `%s` of the copied `label`) captured via `dup2` and compared byte-for-byte between C and Rust, over the row-35 corner grid and random inputs | [x] |
| 38 | composed pipeline | `overunder`'s internal call chain re-driven manually through the low-level exports (`safe_double_to_int`, `process_with_fallthrough`, `handle_pointer_operations`, `copy_data_block`) and cross-checked against `overunder`'s return value, for random inputs — catches divergence in the composition, not just per-wrapper | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the default build is
the only configuration. Verified mechanically by `check_features.sh`.

Rows 1–36 and 38 are implemented in `tests/phase_b_configs.rs` (functions
`row01_*` … `row38_*`, one test per row). Row 37 is `tests/phase_d_stdout.rs`.

## Negative controls

A configuration table that is never exercised proves nothing, so
`negative_controls.sh` injects **27 behaviour-changing mutations** into
`src/lib.rs` (one at a time, original restored afterwards) and requires the full
suite to fail for each. All 27 are detected. Mutations cover: every arm of the
`switch` fall-through chain, the `default` sentinel, both `safe_double_to_int`
guards and their comparison direction, the NaN branch and its position in the
if-chain, truncation vs rounding, the `memcpy` length, an added null check, the
`printf` format strings and literals, each floating-point constant (1.5 / 2.7 /
3.3), the `sqrt` operand's integer width, `%` vs `rem_euclid`, argument
mix-ups, and the `strncpy` zero-padding.

Two mutations were found to be **equivalent** rather than blind spots, and are
documented in the script: changing the high guard from `>` to `>=` does not
change any result, because at `d == (double)INT_MAX` the guard returns `INT_MAX`
and the `(int)d` else-branch also yields `2147483647` (verified directly against
the C). The same holds for the low guard at `d == (double)INT_MIN`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the default build is
the only configuration. Verified mechanically by `check_features.sh`, which
enumerates the `[features]` keys, builds each combination, diffs `nm -D` against
the C `.so`, and runs the whole suite. The suite is additionally run against the
`dev`-profile Rust `.so` (overflow checks on, no LLVM optimization) via
`RUST_SO=target/debug/liboverunder_lib.so`.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` symbol diff is EMPTY (5/5 C symbols exported by
      Rust); 0 undefined non-libc symbols.
- [x] Phase B: all 38 `CONFIGS.md` rows pass across randomized inputs
      (fixed seed, 37 tests).
- [x] Phase C: all 14 `ERRORS.md` rows have a passing error-path differential
      test asserting the exact C sentinel (15 tests).
- [x] Every call in every test goes through a `.so` export loaded with
      `libloading` — for both libraries. No Rust function is called directly and
      the crate is not linked into the tests (`crate-type = ["cdylib"]` only).
- [x] Holds under every feature combination (there is exactly one) and under
      both the `release` and `dev` Rust profiles.
- [x] The suite is proven able to fail: 27/27 injected mutations detected.
