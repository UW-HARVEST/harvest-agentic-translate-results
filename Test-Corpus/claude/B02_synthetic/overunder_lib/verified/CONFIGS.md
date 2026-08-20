# CONFIGS.md — Configuration / valid-input surface table

## Build-time configuration axes

| axis | values | notes |
|------|--------|-------|
| Cargo `[features]` | **none declared** | `Cargo.toml` has no `[features]` table, so the *only* feature combination is the empty one. `cargo check/test --no-default-features` ≡ `cargo check/test`. Verified by `scripts/check_all_features.sh`. |
| CMake options | **none declared** | `c_src/CMakeLists.txt` has no `option()` / `add_definitions()` / generator expressions. Single `SHARED` target from `src/lib.c`, `-lm`. |
| `#ifdef` in C source | **none** | `grep -c '#if' c_src/src/lib.c` → 0. The only preprocessor use is the two object-like/function-like macros `MAKE_VAR_NAME` and `PRINT_VAR`, which are unconditional. |

⇒ **1 build configuration.** All rows below are therefore verified under the one
and only feature combination, which is simultaneously the default.

## Runtime configuration axes (derived from the branches the C actually takes)

There is no init function, no handle/context object, no option setter and no
global mutable state. The public API is four leaf functions plus one composite
driver, so the "configuration" of a call *is* its argument shape. The axes the C
branches on are:

* **A1** `safe_double_to_int`: which of the 4 arms (`>INT_MAX`, `<INT_MIN`,
  `isnan`, `(int)d`) is taken, and for the 4th arm the truncation direction
  (toward zero ⇒ down for positive, up for negative).
* **A2** `process_with_fallthrough`: which of the 8 `switch` arms (`5`,`4`,`3`,
  `2`,`1`,`0`,`default`) is entered, and how far the fall-through chain runs.
* **A3** `copy_data_block`: the 40-byte payload shape, **including the 4 padding
  bytes at offset 4 and the 4 tail-padding bytes at offset 36**, which `memcpy`
  copies but no field access can observe.
* **A4** `handle_pointer_operations`: `value * 2` in range vs. overflowing.
* **A5** `overunder` composite: `a % 6` selecting the A2 arm; sign of `a`,`b`,
  `c`,`d`; whether `d*d + a*a` overflows `int` (⇒ `sqrt` of a negative ⇒ NaN);
  whether `a*1.5` / `b*2.7` reach the A1 clamps; the sign/magnitude of `c/3.3`;
  whether `a+b` overflows; whether the running `total` overflows; and the
  `%.2f` / `%s` / `%d` formatting of stdout.

`overunder` is the convenience/one-shot wrapper; rows C1–C24 exercise the four
**low-level** entry points directly, and rows C25–C40 exercise `overunder`.

Every row is driven with **many randomized inputs (fixed seed `0x5EED_1234_ABCD_F00D`,
SplitMix64)** in the shape the row describes, not a single hand-picked value, and
compares the C `.so` and Rust `.so` return values (and, for `overunder`, the
captured stdout bytes) for exact equality.

## `safe_double_to_int` — entry point 1/5

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| C1 | `safe_double_to_int` | arm 4, `d == 0.0` and `d == -0.0` (signed-zero shape) | `cfg_c1_signed_zero` | [x] |
| C2 | `safe_double_to_int` | arm 4, subnormal and tiny magnitudes `\|d\| < 1` (`±5e-324`, `±1e-300`, `±0.5`, `±0.9999999999`) — truncate-toward-zero must give `0`/`-0` | `cfg_c2_subnormal_and_fractional` | [x] |
| C3 | `safe_double_to_int` | arm 4, **positive** in-range fractional values, randomized in `(0, INT_MAX)` — truncation rounds *down* | `cfg_c3_positive_inrange_random` | [x] |
| C4 | `safe_double_to_int` | arm 4, **negative** in-range fractional values, randomized in `(INT_MIN, 0)` — truncation rounds *up* | `cfg_c4_negative_inrange_random` | [x] |
| C5 | `safe_double_to_int` | arm 4, exact integral doubles across the whole `int` range (no fractional part) | `cfg_c5_exact_integral_random` | [x] |
| C6 | `safe_double_to_int` | arms 1+2, magnitudes just outside the range, randomized in `(INT_MAX, 2^40)` and `(-2^40, INT_MIN)` | `cfg_c6_just_outside_range` | [x] |
| C7 | `safe_double_to_int` | arm 4, values within 1 ULP of the two range boundaries (`nextafter` ladder, 8 steps either side of `±2147483648`) | `cfg_c7_ulp_ladder_at_boundaries` | [x] |
| C8 | `safe_double_to_int` | all arms, **arbitrary 64-bit bit patterns reinterpreted as `double`** (uniform random `u64` → `f64`; covers NaN payloads, infinities, huge/tiny exponents in one sweep) | `cfg_c8_arbitrary_bit_patterns` | [x] |

## `process_with_fallthrough` — entry point 2/5

| #   | entry point(s) | configuration (options set + input shape) | test | [x] |
|-----|----------------|-------------------------------------------|------|-----|
| C9  | `process_with_fallthrough` | `code == 5` (full 5-deep fall-through, `+150`) × `base_value` ∈ {0, small +, small −, random} | `cfg_c9_c14_each_case_random` | [x] |
| C10 | `process_with_fallthrough` | `code == 4` (4-deep, `+100`) × same `base_value` shapes | `cfg_c9_c14_each_case_random` | [x] |
| C11 | `process_with_fallthrough` | `code == 3` (3-deep, `+60`) × same `base_value` shapes | `cfg_c9_c14_each_case_random` | [x] |
| C12 | `process_with_fallthrough` | `code == 2` (2-deep, `+30`) × same `base_value` shapes | `cfg_c9_c14_each_case_random` | [x] |
| C13 | `process_with_fallthrough` | `code == 1` (`break`, `+10`) × same `base_value` shapes | `cfg_c9_c14_each_case_random` | [x] |
| C14 | `process_with_fallthrough` | `code == 0` (`result = 0`, base discarded) × same `base_value` shapes | `cfg_c9_c14_each_case_random` | [x] |
| C15 | `process_with_fallthrough` | `default` arm × full random `base_value` (result must be `-1` independent of base) | `cfg_c15_default_arm_random` | [x] |
| C16 | `process_with_fallthrough` | full cross-product: `code` ∈ `[-8, 13]` × `base_value` ∈ {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-150`, `INT_MAX-1`, `INT_MAX`} (176 combinations, exhaustive) | `cfg_c16_exhaustive_cross_product` | [x] |
| C17 | `process_with_fallthrough` | `code` fully random `i32` (mostly `default`) × `base_value` fully random `i32` | `cfg_c17_fully_random` | [x] |

## `copy_data_block` — entry point 3/5

| #   | entry point(s) | configuration (options set + input shape) | test | [x] |
|-----|----------------|-------------------------------------------|------|-----|
| C18 | `copy_data_block` | all-zero 40-byte source | `cfg_c18_c20_payload_shapes` | [x] |
| C19 | `copy_data_block` | all-`0xFF` 40-byte source (NaN in `value`, no NUL in `label`) | `cfg_c18_c20_payload_shapes` | [x] |
| C20 | `copy_data_block` | fully random 40 bytes, **including the padding at offsets 4..8 and 36..40** — asserts the padding is copied too | `cfg_c18_c20_payload_shapes` | [x] |
| C21 | `copy_data_block` | struct-typed shapes: random `id`, random `value` (incl. `NaN`/`±inf`/subnormal), `label` = empty / 6-byte / 19-byte / 20-byte-no-NUL | `cfg_c21_struct_typed_shapes` | [x] |
| C22 | `copy_data_block` | dest and src at **different alignments/offsets inside a heap arena** (offsets 0 and 8, both 8-aligned as the ABI requires), many random payloads | `cfg_c22_arena_offsets` | [x] |
| C23 | `copy_data_block` | dest buffer pre-filled with a distinct sentinel, checked that exactly `[0,40)` changed and the surrounding 56 sentinel bytes did not | `err_e18_copies_exactly_40_bytes` | [x] |
| C24 | `handle_pointer_operations` | `value` ∈ {0, ±1, ±small, `INT_MAX/2`, `INT_MAX/2+1`, `INT_MIN/2`, `INT_MIN/2-1`, `INT_MAX`, `INT_MIN`} ∪ 2000 random `i32` | `cfg_c24_hpo_full_range` | [x] |

## `overunder` — entry point 4/5 (composite driver), and its 4-axis cross-product

| #   | entry point(s) | configuration (options set + input shape) | test | [x] |
|-----|----------------|-------------------------------------------|------|-----|
| C25 | `overunder` | `a % 6 == 0` (`a` ∈ {0, 6, 12, …}) — drives A2 arm `case 0`, `switch_result == 0` | `cfg_c25_c30_modulo_arms` | [x] |
| C26 | `overunder` | `a % 6 == 1` — drives A2 arm `case 1` | `cfg_c25_c30_modulo_arms` | [x] |
| C27 | `overunder` | `a % 6 == 2` — drives A2 arm `case 2` (2-deep fall-through) | `cfg_c25_c30_modulo_arms` | [x] |
| C28 | `overunder` | `a % 6 == 3` — drives A2 arm `case 3` (3-deep) | `cfg_c25_c30_modulo_arms` | [x] |
| C29 | `overunder` | `a % 6 == 4` — drives A2 arm `case 4` (4-deep) | `cfg_c25_c30_modulo_arms` | [x] |
| C30 | `overunder` | `a % 6 == 5` — drives A2 arm `case 5` (5-deep) | `cfg_c25_c30_modulo_arms` | [x] |
| C31 | `overunder` | `a < 0` with `a % 6` ∈ {-1,…,-5} — drives A2 `default` arm (C `%` truncates toward zero) | `cfg_c31_negative_modulo_arms` | [x] |
| C32 | `overunder` | all-zero call `(0,0,0,0)` — the "empty" shape: `sqrt(0)`, `0/3.3`, `label=Source`, `total` from constants only | `cfg_c32_all_zero` | [x] |
| C33 | `overunder` | small positive quadruples (`0..64` each), exhaustive-ish random sweep — no overflow anywhere | `cfg_c33_small_positive` | [x] |
| C34 | `overunder` | mixed signs: the full 16-way sign cross-product of `(±a, ±b, ±c, ±d)` with randomized magnitudes in `[1, 10^4]` | `cfg_c34_sign_cross_product` | [x] |
| C35 | `overunder` | `d*d + a*a` **does not** overflow (`\|a\|,\|d\| <= 32767`) — `sqrt` of a non-negative value, `conv4 > 0` | `cfg_c35_sqrt_no_overflow` | [x] |
| C36 | `overunder` | `d*d + a*a` **overflows** `int` (`\|a\|` or `\|d\| >= 46341`) — wrapped sum may be positive *or* negative ⇒ `sqrt` of negative ⇒ NaN ⇒ `conv4 == 0`; both sub-shapes covered | `cfg_c36_sqrt_overflow_both_signs` | [x] |
| C37 | `overunder` | `a*1.5` and/or `b*2.7` reach the A1 clamps. Thresholds are **asymmetric**: `a >= 1431655765` (high), `a <= -1431655766` (low — `-1431655765*1.5 == -2147483647.5` is *not* `< INT_MIN`), `b >= 795364314` (high), `b <= -795364315` (low). All 4 sign sub-shapes × randomized jitter, plus each threshold and one step inside it. | `cfg_c37_internal_clamps` | [x] |
| C38 | `overunder` | `c` shapes for `c/3.3`: `c` ∈ {0, ±1, ±3, ±4, ±INT_MAX, ±INT_MIN}, and `c` near `INT_MAX/2` so `handle_pointer_operations(c)` overflows | `cfg_c38_c_division_and_ptr_shapes` | [x] |
| C39 | `overunder` | extreme corners: every one of the 16 combinations of `{INT_MIN, INT_MAX}` for `(a,b,c,d)`, plus `INT_MIN+1`/`INT_MAX-1` variants — `a+b` overflow, `total` overflow, `INT_MIN % 6` | `cfg_c39_extreme_corners` | [x] |
| C40 | `overunder` | fully random `i32` quadruples (4000 iterations) — unconstrained cross-product of all of the above axes, comparing **both** the `int` return value **and** the byte-exact captured stdout | `cfg_c40_fully_random_with_stdout` | [x] |

## Cross-cutting axes verified on every `overunder` row

| axis | how it is checked |
|------|-------------------|
| return value | `int` equality between the two `.so`s |
| stdout bytes | fd 1 is redirected to a temp file around each call, `fflush(NULL)`ed, and the two byte vectors compared with `assert_eq!` (covers the `PRINT_VAR` format strings, `%d`, `%.2f` of `value`, `%s` of `label`, and the trailing `"%d "` loop + newline) |
| `DataBlock` padding | `copy_data_block` rows compare all 40 bytes, not fields |
| repeated invocation | each row calls C then Rust in the same process, so any hidden global state would show up as a second-call divergence |
