# ERRORS.md — Error / rejection surface of `c_src/src/lib.c`

## How this table was derived

Mechanical greps over the single C translation unit (`c_src/src/lib.c`):

```
grep -n 'return' lib.c
grep -nE 'assert|RETURN_ERROR|errno|NULL|-1|goto|abort|exit' lib.c
grep -nE 'if|else|switch|case|default|for|while|%' lib.c
```

Findings that shape the table:

- **No** `assert`, **no** `RETURN_ERROR`-style macro, **no** `errno` use, **no**
  `goto`/`abort`/`exit`, **no** `return -1`, **no** `return NULL`, **no** error
  `enum`, and **no** explicit range check or min/max constant anywhere.
- Every `return` in the file is a value return, never a sentinel.

Therefore this library's rejection surface consists of exactly two *explicit*
fall-through rejections plus a set of *implicit* rejections where the C relies on
undefined behaviour that the hardware resolves deterministically. Both classes
get a row, because both are real inputs a caller can pass across the FFI
boundary and both must behave identically in Rust.

Sentinel legend:
- `INT_MIN` = `-2147483648` = `0x80000000`, the x86-64 `cvttsd2si` "integer
  indefinite" result produced for every `(int)` cast whose truncation does not
  fit in `int` (lib.c:66, lib.c:74).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E1  | `classify_mode` | string matching none of the four literals (lib.c:39 terminal `return 0x00`) | `0x00` | [x] |
| E2  | `classify_mode` | empty string `""` | `0x00` | [x] |
| E3  | `classify_mode` | strict prefix of a valid mode, e.g. `"standar"`, `"turb"` (strcmp != 0) | `0x00` | [x] |
| E4  | `classify_mode` | valid mode plus trailing bytes, e.g. `"standardX"`, `"turbo "` | `0x00` | [x] |
| E5  | `classify_mode` | case variant of a valid mode, e.g. `"Standard"`, `"TURBO"` | `0x00` | [x] |
| E6  | `classify_mode` | string with embedded NUL after a valid mode, e.g. `"turbo\0zzz"` — `strcmp` stops at NUL | `0x30` (accepted, NOT rejected) | [x] |
| E7  | `classify_mode` | non-ASCII / high-bit bytes (`\xff\xfe`) — `strcmp` compares as `unsigned char` | `0x00` | [x] |
| E8  | `classify_mode` | `NULL` pointer | UB: `strcmp(NULL, …)` dereferences null → SIGSEGV. Not a comparable return value; documented, not asserted. | [x] (documented) |
| E9  | `apply_multiplier` | `level < 0` (any negative), hits `default:` at lib.c:57 | `0xDEAD` = `57005`, base ignored | [x] |
| E10 | `apply_multiplier` | `level > 4` (any value ≥ 5), hits `default:` | `0xDEAD` = `57005`, base ignored | [x] |
| E11 | `apply_multiplier` | `level == INT_MIN` / `level == INT_MAX` (extreme out-of-range enum-ish value across FFI) | `0xDEAD` = `57005` | [x] |
| E12 | `apply_multiplier` | in-range `level` but `base` such that the accumulated `+=` overflows `int` (e.g. `base = INT_MAX`, `level = 4`) | UB signed overflow; gcc wraps two's-complement | [x] |
| E13 | `convert_time_factor` | `factor * 1e12` truncates above `INT_MAX` (e.g. `factor = 1.0`) | `INT_MIN` | [x] |
| E14 | `convert_time_factor` | `factor * 1e12` truncates below `INT_MIN` (e.g. `factor = -1.0`) | `INT_MIN` | [x] |
| E15 | `convert_time_factor` | `factor = NaN` | `INT_MIN` | [x] |
| E16 | `convert_time_factor` | `factor = +INFINITY` / `-INFINITY` | `INT_MIN` | [x] |
| E17 | `convert_time_factor` | `factor` so large that `factor * 1e12` itself overflows to `inf` (e.g. `DBL_MAX`) | `INT_MIN` | [x] |
| E18 | `convert_time_factor` | value exactly one step past the valid range: `factor` where `factor*1e12` == `2147483648.0` and `== -2147483649.0` | `INT_MIN` (whereas `2147483647.0` / `-2147483648.0` are accepted) | [x] |
| E19 | `convert_negative_overflow` | `value * -1e15` truncates outside `int` range (e.g. `value = 1.0`, `value = -1.0`) | `INT_MIN` | [x] |
| E20 | `convert_negative_overflow` | `value = NaN` | `INT_MIN` | [x] |
| E21 | `convert_negative_overflow` | `value = ±INFINITY`, `±DBL_MAX` | `INT_MIN` | [x] |
| E22 | `convert_negative_overflow` | `value = -0.0` → `extreme = +0.0`; `value = 0.0` → `extreme = -0.0` | `0` in both cases (accepted, sign of zero truncates to 0) | [x] |
| E23 | `convert_negative_overflow` | one step past range: `value` where `value*-1e15` == `2147483648.0` / `-2147483649.0` | `INT_MIN` | [x] |
| E24 | `get_modified_time` | `offset_days * 86400` overflows `int` (e.g. `offset_days = 100000`, `INT_MAX`, `INT_MIN`) | UB signed overflow; gcc wraps in `int`, then sign-extends to `time_t` | [x] |
| E25 | `get_modified_time` | `offset_hours * 3600` overflows `int` (e.g. `offset_hours = INT_MAX`) | same: wraps in `int`, then sign-extends | [x] |
| E26 | `get_modified_time` | the `+` of the two wrapped products itself overflows `int` | wraps in `int`, then sign-extends | [x] |
| E27 | `hash_time_value` | `t` with high-bit bytes so `bytes[i] << 24` overflows signed `int` (lib.c:90) | UB shift overflow; gcc wraps. Result always masked to `0..=0x7FFFFFFF` | [x] |
| E28 | `hash_time_value` | `hash *= 0x1F` overflows signed `int` on every one of the 8 iterations | UB signed overflow; gcc wraps two's-complement | [x] |
| E29 | `modeselect` | `mode_selector < 0` and not a multiple of 4 → `mode_index` is `-1/-2/-3` → out-of-bounds read of the 4-element stack array `modes` at lib.c:99–102, then `strcmp` on the garbage pointer | UB: **verified to SIGSEGV** (see note below). Not a comparable return value; documented, not asserted. | [x] (documented) |
| E30 | `modeselect` | `complexity % 5` negative (any `complexity < 0` not a multiple of 5) → `apply_multiplier` `default:` | `multiplier == 0xDEAD`; propagates into `result` | [x] |
| E31 | `modeselect` | `mode_selector == INT_MIN` → `INT_MIN % 4 == 0`, so index 0 is in range | accepted, mode `"standard"` → `0x10` | [x] |
| E32 | `modeselect` | `seed % 24` negative → negative `offset_hours` into `get_modified_time` | accepted; negative offset | [x] |
| E33 | `modeselect` | `time_hash % 0x1000` — `time_hash` is masked non-negative so this never goes negative | accepted | [x] |

## Note on E8 and E29 (uncomparable UB)

These two rows are the only rejections whose C behaviour is a **process crash**
rather than a return value:

- E8: `classify_mode(NULL)` → `strcmp` dereferences a null pointer.
- E29: verified empirically with a `dlopen` probe against the real C `.so`:
  `modeselect(-1, 0, 0, 0)` terminates the process with
  `Segmentation fault (core dumped)` (exit 139).

A differential test cannot assert "both returned the same error code" when one
side has no return value at all, and deliberately making the Rust crash would
make the library strictly worse without making it more faithful. Both rows are
therefore recorded as documented-and-excluded rather than asserted, and the tests
`e8_classify_mode_null_is_documented_ub` and
`e29_modeselect_negative_selector_segfaults_in_c` encode that reasoning (the
latter proves the C crashes by running the call in a forked child and checking
for `SIGSEGV`, and proves the Rust does not crash).

For E29 the Rust substitutes an empty mode string, which routes to the same
`classify_mode` result of `0x00` the crash-free portion of the C would have
produced. Every non-crashing `mode_selector` (all `>= 0`, plus negative
multiples of 4 including `INT_MIN`) is covered by asserted differential rows.

## Where each row is tested

`translation/tests/phase_c_errors.rs`, one test per row, named after the row:

| rows | test |
|------|------|
| E1–E7 | `e1_..` … `e7_classify_mode_high_bit_bytes` |
| E8 | `e8_classify_mode_null_is_documented_ub` (fork + `SIGSEGV` check) |
| E9–E12 | `e9_..` … `e12_apply_multiplier_accumulator_overflow` |
| E13–E18 | `e13_..` … `e18_convert_time_factor_one_step_past_range` |
| E19–E23 | `e19_..` … `e23_convert_negative_overflow_one_step_past_range` |
| E24–E26 | `e24_..` … `e26_get_modified_time_sum_overflows` |
| E27–E28 | `e27_hash_time_value_high_bit_shift_overflow`, `e28_hash_time_value_multiply_overflow` |
| E29 | `e29_modeselect_negative_selector_segfaults_in_c` (fork + `SIGSEGV` check) |
| E30–E33 | `e30_..` … `e33_modeselect_time_hash_modulo_never_negative` |

Plus the generic FFI-boundary rows the table does not enumerate:

- `generic_out_of_range_enum_values_across_ffi` — 20 000 randomized `int`s into
  `apply_multiplier`'s `switch`, the only enum-like dispatch. C enums accept any
  `int`, so every value with no matching `case` is a real input; all must give
  `0xDEAD` on both sides.
- `generic_zero_and_oversized_lengths` — empty string, 64 KiB strings, and a
  literal followed by NUL padding.
- `generic_one_step_past_every_documented_range` — one step past each of
  `level ∈ 0..=4`, `mode_index ∈ 0..=3`, `complexity_level ∈ 0..=4`,
  `seed % 24`, and both `(int)double` boundaries.

Every row asserts the SPECIFIC sentinel (`0x00`, `0xDEAD`, `INT_MIN`, or the
exact wrapped value), not merely that both sides failed.

## Corrections made while building this table

Two rows were initially recorded with the wrong expected C value. The C was
right both times; the table and test were fixed, never the C:

- **E12**: the level-4 fall-through total was written as `0x24D` (589). It is
  `0xFF + 0xAB + 0x7E + 0x1C + 0x05 = 585 = 0x249`.
- **E30**: `-100` was listed as reaching the `default:` arm. `-100 % 5 == 0` in
  C, so it reduces to level 0, which is in range. Moved to the accepted group.

## Mutation evidence

An error table is worthless if the tests behind it cannot fail.
`mutation_check.py` injects 43 deliberate behavioural bugs into
`translation/src/lib.rs` — wrong sentinels (`0xDEAD` → `0xDEAF`), saturating
instead of indefinite `(int)double` casts, NaN handling, dropped `switch`
fall-through, `i64` instead of wrapping `i32` offset arithmetic, wrong hash
seed/multiplier/mask/byte-order, prefix-only `strcmp`, and each `printf` format
change — rebuilds, and requires the suite to reject each one.

**Result: 43/43 caught.**

Three known EQUIVALENT mutants are deliberately excluded, with the reason
recorded and evidence in
`tests/phase_b_modeselect.rs::invariant_modeselect_cast_results_are_only_zero_or_int_min`:

1. `result1 & 0xFF` → `& 0xFFF` and 2. `result2 & 0xFF00` → `& 0xF000`. Inside
   `modeselect`, `factor1 = seed * 1e8` is scaled by a further `1e12` and
   `factor2 = time_offset * -1e7` by `-1e15`, so both casts can only ever yield
   `0` or `INT_MIN` (`0x80000000`). Every low-bit mask of those two values is 0,
   so both XOR terms are unconditionally no-ops and no mask width is observable.
   The XOR *operations* are mutated instead, and both are caught.
3. `mode_selector % 4` → `.rem_euclid(4)`. These differ only for negative
   selectors, and every negative non-multiple of 4 makes the C SIGSEGV (E29), so
   the difference cannot be observed against the ground truth at all.
