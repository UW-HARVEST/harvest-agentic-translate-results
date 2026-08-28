# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived from `c_src/src/lib.c`.  Greps performed:

```
grep -n 'return'  c_src/src/lib.c   # 8 return statements, 0 error macros
grep -n 'assert'  c_src/src/lib.c   # none
grep -n 'NULL'    c_src/src/lib.c   # only time(NULL)
grep -n 'errno\|RETURN_ERROR\|goto\|exit(' c_src/src/lib.c   # none
grep -n 'enum'    c_src/src/lib.c c_src/include/lib.h        # none
```

**This library has no error-code convention at all**: no `errno`, no `-1`
sentinel returns, no `NULL` returns, no `assert`, no enums, no explicit range
checks.  Its "rejection surface" therefore consists of

* the two **`else`/`default` fallback sentinels** (`0x00` from `classify_mode`,
  `0xDEAD` from `apply_multiplier`),
* the **saturating/indefinite results of undefined-behaviour operations** the C
  performs unguarded (`double`→`int` out-of-range casts, signed `int` overflow,
  signed left-shift overflow),
* and the two **hard faults** reachable from the public API (`NULL` string
  dereference in `strcmp`, negative array index in `modeselect`).

Every one of those is a real input an external caller can supply, so every one
gets a row and a differential test.

## Table

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| E1 | `classify_mode` | `mode` is a NUL-terminated string equal to none of `"standard"`, `"enhanced"`, `"turbo"`, `"extreme"` (falls off the `else if` chain, `lib.c:38`) | returns `0x00` |
| E2 | `classify_mode` | `mode` is the empty string `""` | returns `0x00` |
| E3 | `classify_mode` | `mode` is a strict *prefix* of a valid mode (`"stand"`, `"turb"`, `"e"`) | returns `0x00` |
| E4 | `classify_mode` | `mode` is a valid mode with extra trailing bytes (`"standardX"`, `"turbo "`) | returns `0x00` |
| E5 | `classify_mode` | `mode` differs from a valid mode only in case (`"Standard"`, `"TURBO"`) | returns `0x00` |
| E6 | `classify_mode` | `mode` contains bytes ≥ 0x80 (`strcmp` compares as `unsigned char`) | returns `0x00` |
| E7 | `classify_mode` | `mode == NULL` — `strcmp(NULL, "standard")` dereferences the null page (`lib.c:30`) | process dies with `SIGSEGV` (11) |
| E8 | `apply_multiplier` | `level < 0` (e.g. `-1`, `-4`, `INT_MIN`) → `switch` `default:` (`lib.c:58`) | returns `0xDEAD` (57005), `base` ignored |
| E9 | `apply_multiplier` | `level > 4` (e.g. `5`, `100`, `INT_MAX`) → `switch` `default:` | returns `0xDEAD` (57005), `base` ignored |
| E10 | `apply_multiplier` | `level` in `0..=4` **but** `base` near `INT_MAX` so `result += …` overflows signed `int` (UB, `lib.c:46-56`) | wraps modulo 2³² (gcc two's-complement wraparound) |
| E11 | `convert_time_factor` | `factor * 1e12 > INT_MAX` (i.e. `factor > 2.147483647e-3`) → out-of-range `(int)` cast (`lib.c:67`) | `cvttsd2si` "integer indefinite" = `-2147483648` (`0x80000000`) |
| E12 | `convert_time_factor` | `factor * 1e12 < INT_MIN` (i.e. `factor < -2.147483648e-3`) | `-2147483648` (`0x80000000`) |
| E13 | `convert_time_factor` | `factor` is `NaN` (either sign / any payload) | `-2147483648` |
| E14 | `convert_time_factor` | `factor` is `+INFINITY` or `-INFINITY` | `-2147483648` |
| E15 | `convert_negative_overflow` | `value * -1e15 > INT_MAX` (i.e. `value < -2.147483647e-6`) → out-of-range cast (`lib.c:74`) | `-2147483648` (`0x80000000`) |
| E16 | `convert_negative_overflow` | `value * -1e15 < INT_MIN` (i.e. `value > 2.147483648e-6`) | `-2147483648` (`0x80000000`) |
| E17 | `convert_negative_overflow` | `value` is `NaN` | `-2147483648` |
| E18 | `convert_negative_overflow` | `value` is `±INFINITY` (product is `∓INFINITY`) | `-2147483648` |
| E19 | `get_modified_time` | `offset_days * 86400` overflows `int` (`|offset_days| > 24855`, UB at `lib.c:82`) | 32-bit wraparound, then sign-extended to `time_t` |
| E20 | `get_modified_time` | `offset_hours * 3600` overflows `int` (`|offset_hours| > 596523`) | 32-bit wraparound, then sign-extended |
| E21 | `get_modified_time` | the *sum* `(days*86400)+(hours*3600)` overflows `int` although neither product does | 32-bit wraparound, then sign-extended |
| E22 | `hash_time_value` | `t` has any byte ≥ 0x80 in position `i%4 == 3`, so `bytes[i] << 24` overflows signed `int` (UB at `lib.c:91`) | two's-complement bit pattern kept; final `& 0x7FFFFFFF` always ≥ 0 |
| E23 | `hash_time_value` | `t < 0` / `t == INT64_MIN` / `t == INT64_MAX` | still returns a value in `[0, 0x7FFFFFFF]` (never negative) |
| E24 | `modeselect` | `mode_selector < 0` and `mode_selector % 4 != 0` → `mode_index` is `-1`, `-2` or `-3` → `modes[mode_index]` reads *below* the stack array (`lib.c:104-105`), then `strcmp` on the garbage pointer | **UNSPECIFIED** — the value read is uninitialised stack memory, so the result is a function of the *caller's* stack, not of the input (see "E24 is not a contract" below).  With the stack region zeroed the read yields `NULL` and both libraries die with `SIGSEGV` (11); that is the sub-case the differential test asserts. |
| E25 | `modeselect` | `mode_selector < 0` and `mode_selector % 4 == 0` (`-4`, `-8`, `INT_MIN`) | index `0`, behaves exactly like `mode_selector == 0` |
| E26 | `modeselect` | `complexity < 0` and `complexity % 5 != 0` → `complexity_level` negative → `apply_multiplier` `default:` | multiplier is `0xDEAD`; `result` accumulates 57005 |
| E27 | `modeselect` | any `seed != 0`: `factor1 = seed*1e8`, `convert_time_factor` multiplies by `1e12` → `|seed|*1e20` always out of `int` range | `result1 == -2147483648`; `result1 & 0xFF == 0` |
| E28 | `modeselect` | any `time_offset != 0`: `factor2 = time_offset*-1e7`, `convert_negative_overflow` multiplies by `-1e15` → `|time_offset|*1e22` out of range | `result2 == -2147483648`; `result2 & 0xFF00 == 0` |
| E29 | `modeselect` | `result * 0x10 + 0xBEEF` overflows signed `int` (`lib.c:137`) | **PROVABLY UNREACHABLE**: `mode_value ≤ 0x40`, `multiplier ≤ 0xDEAD`, `time_hash % 0x1000 ≤ 0xFFF`, and both xor masks are always `0` (E27/E28), so `result ≤ 0xEEEC` and `result*0x10 + 0xBEEF ≤ 0xFF00F`. The test asserts the returned value stays inside that provable range for both libraries. |
| E30 | *(generic FFI boundary)* | out-of-range "enum" values: the API has **no enum parameters**; the closest analogue is `apply_multiplier`'s `level` and `modeselect`'s `mode_selector`/`complexity` discriminants, covered by E8/E9/E24/E25/E26. Rows kept explicit so the check is not silently skipped. | see E8, E9, E24, E25, E26 |

## Row → test mapping (all rows PASSING)

Every row below has a differential test in `tests/phase_c_errors.rs` that
constructs the exact condition, calls **both** `.so`s through their exported C
ABI, and asserts the same error/sentinel value (not merely "both failed").

| row | test function | [x] |
|---|---|---|
| E1  | `e1_classify_unknown_strings` | [x] |
| E2  | `e2_classify_empty_string` | [x] |
| E3  | `e3_classify_strict_prefixes` | [x] |
| E4  | `e4_classify_trailing_bytes` | [x] |
| E5  | `e5_classify_case_variants` | [x] |
| E6  | `e6_classify_high_bytes` | [x] |
| E7  | `e7_classify_null_pointer_segfaults_identically` (forked; compares signal) | [x] |
| E8  | `e8_apply_negative_level` | [x] |
| E9  | `e9_apply_level_above_four` | [x] |
| E10 | `e10_apply_accumulator_signed_overflow` | [x] |
| E11 | `e11_ctf_overflow_positive` | [x] |
| E12 | `e12_ctf_overflow_negative` | [x] |
| E13 | `e13_ctf_nan` (6 NaN payloads, both signs, quiet + signalling) | [x] |
| E14 | `e14_ctf_infinity` | [x] |
| E15 | `e15_cno_overflow_via_negative_input` | [x] |
| E16 | `e16_cno_overflow_via_positive_input` | [x] |
| E17 | `e17_cno_nan` | [x] |
| E18 | `e18_cno_infinity` | [x] |
| E19 | `e19_gmt_days_product_overflow` | [x] |
| E20 | `e20_gmt_hours_product_overflow` | [x] |
| E21 | `e21_gmt_sum_overflow_only` | [x] |
| E22 | `e22_hash_high_bit_in_top_lane` | [x] |
| E23 | `e23_hash_never_negative` | [x] |
| E24 | `e24a_negative_index_with_zeroed_stack_faults_identically`, `e24b_negative_index_pipeline_agrees_for_every_possible_mode_value`, `e24c_negative_index_is_caller_dependent_in_the_c_library` | [x] |
| E25 | `e25_modeselect_negative_multiple_of_four` | [x] |
| E26 | `e26_modeselect_negative_complexity` | [x] |
| E27 | `e27_modeselect_result1_is_int_min_unless_seed_zero` | [x] |
| E28 | `e28_modeselect_result2_is_int_min_unless_time_offset_zero` | [x] |
| E29 | `e29_final_multiply_never_overflows` | [x] |
| E30 | `e30_out_of_range_discriminants` | [x] |

Generic FFI boundaries required in addition to the table above:

| boundary | test function | [x] |
|---|---|---|
| null pointer | `e7_classify_null_pointer_segfaults_identically` | [x] |
| zero length / oversized length (1 MiB) | `generic_zero_and_oversized_string_lengths` | [x] |
| one step past every documented range (incl. `level` = −1 / 5, `double`→`int` edge ±4 ULP, `offset_days` = ±24856, `offset_hours` = ±596524, `time_t` extremes) | `generic_one_past_valid_ranges` | [x] |
| out-of-range "enum"/discriminant ints across FFI | `e30_out_of_range_discriminants` | [x] |
| `INT_MAX` / `INT_MIN` in every parameter position | `generic_extreme_modeselect_arguments` | [x] |

## Notes on faithfulness

* E7 is a genuine and *deterministic* memory-safety fault: `strcmp(NULL, …)`
  always faults.  The differential test executes both libraries in a **forked
  child** and compares the *termination signal*, so "C segfaults ⇒ Rust must
  segfault with the same signal" is asserted rather than glossed over.
* E11–E18: Rust's `as` operator **saturates**, which would diverge from
  `cvttsd2si`.  The translation therefore routes every `double`→`int` conversion
  through `d2i()`, which returns `INT_MIN` for NaN/∞/out-of-range, matching
  x86-64 gcc bit-for-bit.

### E24 is not a contract — measured evidence

`modes` is a four-element array of `const char *` living in `modeselect`'s own
stack frame.  A negative `mode_index` reads the bytes *below* it, which are
whatever the previous use of that stack region left behind.  Measured against
`libharvest-work-lpPs9a.so` — the very same `.so`, the very same argument
`modeselect(-3, 1, 1, 1)`:

| caller context | C library outcome |
|---|---|
| fresh process, called from `main` (standalone driver) | `SIGSEGV` |
| forked child of the test harness, stack left as-is | returns `112527`, prints a garbage mode name and `(0x0)` |
| forked child, ~32 KiB of stack below pre-filled with `0x00…00` | `SIGSEGV` |
| forked child, ~32 KiB of stack below pre-filled with `&"turbo\0"` | returns `113295`, prints `Selected mode: turbo (0x30)` |

The result is therefore **not a function of the arguments**, and no translation
can be required to reproduce it byte-for-byte.  The Rust translation performs the
*structurally identical* operation —
`ptr::read_volatile(modes.as_ptr().offset(mode_index as isize))` on its own
4-element stack array, using `read_volatile` so the optimiser may not exploit the
UB — and the differential test asserts everything that *is* well defined:

1. with the stack region below deterministically zeroed, the OOB read yields
   `NULL` in both libraries and both die with the **same** signal (`SIGSEGV`);
2. every `mode_value` the garbage read could possibly produce (`0x00`, `0x10`,
   `0x20`, `0x30`, `0x40`) is pushed through the remainder of the pipeline using
   the low-level exports of both libraries, and the two must agree exactly — so
   the whole negative-index code path is verified apart from that single
   unspecified load;
3. whenever a run *does* complete, its return value must satisfy the
   `mode_value`-parameterised formula from (2), which is checked for both
   libraries independently.
