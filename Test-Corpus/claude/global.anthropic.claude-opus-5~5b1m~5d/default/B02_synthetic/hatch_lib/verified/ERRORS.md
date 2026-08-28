# ERRORS.md — Phase A error-surface table

## How this table was derived

Mechanical grep over `c_src/src/lib.c` and `c_src/include/lib.h`:

```
grep -nE 'return\s+-|return\s+NULL|assert|errno|RETURN_ERROR|exit\(|abort\(|perror|goto|#ifdef|#if |enum ' c_src/src/lib.c c_src/include/lib.h
  -> no matches (exit 1)
grep -nE '\b(if|for|while|switch)\b' c_src/src/lib.c
  -> 67, 81, 86, 111, 116, 146, 161
```

**Result: this library has no error-return protocol at all.** There is no error
enum, no sentinel return value, no `assert`, no `errno` use, no null-pointer
check, no size/range validation and no `#ifdef` configuration. Every function
returns `int`/`void` unconditionally.

Consequently the "rejection surface" is made entirely of

1. **guard conditions** that silently turn an operation into a no-op
   (`if (shift_by > 0 && shift_by < size)` at line 67,
   `if (shift > 0 && shift < num_records)` at line 111),
2. **loop guards** that make a loop body execute zero times
   (lines 81/86/116),
3. the **implicit `malloc` failure path** (line 79: `malloc(count * sizeof(int))`
   with `count` converted to `size_t`, so any negative `count` requests
   ~2^64 bytes and `malloc` returns `NULL`), and
4. **hard-crash paths** — the C dereferences caller pointers with no validation
   (lines 68/69, 74, 112/117) and calls a caller-supplied function pointer with
   no validation (line 44), so a `NULL`/invalid argument is a `SIGSEGV`.
   These are still inputs the C "handles" (by faulting), and the Rust must fault
   identically, so they are covered by fork/subprocess differential tests that
   compare the terminating signal.

Every row below is one distinct rejection/degenerate branch that the C code
actually takes. `± value` shorthand: the row is exercised with many randomized
values, not one.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `shift_array_data` | `shift_by == 0` (guard `shift_by > 0` false) | no-op: array bytes unchanged, returns `void` | `err_e1_shift_array_shift_zero` | [x] |
| E2 | `shift_array_data` | `shift_by < 0` (guard `shift_by > 0` false), incl. `INT_MIN` | no-op: array bytes unchanged | `err_e2_shift_array_shift_negative` | [x] |
| E3 | `shift_array_data` | `shift_by == size` (guard `shift_by < size` false) | no-op: array bytes unchanged | `err_e3_shift_array_shift_eq_size` | [x] |
| E4 | `shift_array_data` | `shift_by > size` (guard `shift_by < size` false), incl. `INT_MAX` | no-op: array bytes unchanged | `err_e4_shift_array_shift_gt_size` | [x] |
| E5 | `shift_array_data` | `size <= 0` (any `size` in `{INT_MIN,-1,0}`); guard needs `shift_by < size` with `shift_by > 0`, impossible | no-op: array bytes unchanged | `err_e5_shift_array_size_nonpositive` | [x] |
| E6 | `shift_array_data` | `size == 1, shift_by == 1` — boundary one past the last accepted `shift_by` | no-op | `err_e6_shift_array_size_one` | [x] |
| E7 | `shift_array_data` | `arr == NULL` with a *rejecting* `(size, shift_by)` — guard false, pointer never dereferenced | returns normally, no fault | `err_e7_shift_array_null_ptr_guard_false` | [x] |
| E8 | `shift_array_data` | `arr == NULL` with an *accepting* `(size, shift_by)` = `(10, 3)` — `memmove` on `NULL` | `SIGSEGV` (signal 11) | `err_e8_shift_array_null_ptr_guard_true` (subprocess) | [x] |
| E9 | `process_pointer_data` | `ptr == NULL` — `int value = *ptr` with no null check | `SIGSEGV` (signal 11) | `err_e9_process_pointer_null` (subprocess) | [x] |
| E10 | `process_pointer_data` | `multiplier == 0` (degenerate value, result collapses to `global_accumulator`) | `global_accumulator` | `err_e10_process_pointer_zero_multiplier` | [x] |
| E11 | `process_pointer_data` | `*ptr` and `multiplier` chosen so `value * multiplier` overflows `int` (signed-overflow UB, wraps at `-O0`) | wrapped 32-bit product plus `global_accumulator` | `err_e11_process_pointer_overflow` | [x] |
| E12 | `compute_with_dynamic_memory` | `count == 0` → `malloc(0)` (non-`NULL`, zero-size), both loops skipped | returns `0` | `err_e12_cwdm_count_zero` | [x] |
| E13 | `compute_with_dynamic_memory` | `count < 0` → `(size_t)count * 4` ≈ 2^64 → `malloc` returns `NULL`; loops skipped; `free(NULL)` | returns `0` (no crash) | `err_e13_cwdm_count_negative` | [x] |
| E14 | `compute_with_dynamic_memory` | `count == INT_MIN` (the extreme of the negative branch) → `(size_t)INT_MIN * 4 == 0xFFFF_FFFE_0000_0000` → `malloc` returns `NULL`; loop skipped | returns `0` | `err_e14_cwdm_count_int_min` | [x] |
| E15 | `compute_with_dynamic_memory` | `count` large enough that `sum` overflows `int` (e.g. `base` near `INT_MAX`) | wrapped 32-bit sum | `err_e15_cwdm_sum_overflow` | [x] |
| E16 | `get_time_based_value` | `seed` such that `seed * 3600` overflows `int` (`|seed| > 596523`), incl. `INT_MAX`/`INT_MIN` | `(int)((double)(int)(seed*3600) / 100) + seed`, all wrapping | `err_e16_time_seed_overflow` | [x] |
| E17 | `get_time_based_value` | `seed == 0` (degenerate: `diff == 0`) | returns `0` | `err_e17_time_seed_zero` | [x] |
| E18 | `manipulate_records` | `shift == 0` (guard `shift > 0` false); loop runs `num_records` times | sum of all `num_records` `value`s, no memmove | `err_e18_records_shift_zero` | [x] |
| E19 | `manipulate_records` | `shift < 0` (guard false) → loop bound `num_records - shift > num_records` → **reads past the requested range** | sum over `num_records - shift` elements of whatever memory follows | `err_e19_records_shift_negative` (oversized, fully-initialised backing buffer so the over-read is deterministic) | [x] |
| E19b | `manipulate_records` | `(num_records, shift)` whose `num_records - shift` **overflows into a large POSITIVE** bound (e.g. `(-1, INT_MIN)` → `INT_MAX`, `(INT_MIN, 1)` → `INT_MAX`) → the read loop walks ~100 GiB past the buffer | `SIGSEGV` (signal 11) | `err_e23_records_num_minus_shift_overflow`, `err_e22_records_num_nonpositive` (subprocess) | [x] |
| E20 | `manipulate_records` | `shift == num_records` (guard `shift < num_records` false) → loop bound `0` | returns `0`, no memmove | `err_e20_records_shift_eq_num` | [x] |
| E21 | `manipulate_records` | `shift > num_records` (guard false) → loop bound negative → zero iterations | returns `0`, no memmove | `err_e21_records_shift_gt_num` | [x] |
| E22 | `manipulate_records` | `num_records <= 0` (`{INT_MIN,-1,0}`) | guard false, loop bound `<= 0` for `shift >= 0` → returns `0` | `err_e22_records_num_nonpositive` | [x] |
| E23 | `manipulate_records` | `num_records == 0, shift == INT_MIN` → `0 - INT_MIN` overflows `int` (UB, wraps to `INT_MIN`) → loop bound negative → zero iterations | returns `0` | `err_e23_records_num_minus_shift_overflow` | [x] |
| E24 | `manipulate_records` | `records == NULL` with a *rejecting* shape (`num_records <= 0`, `shift == 0`) — pointer never dereferenced | returns `0`, no fault | `err_e24_records_null_ptr_guard_false` | [x] |
| E25 | `manipulate_records` | `records == NULL, num_records = 5, shift = 2` — `memmove` + deref on `NULL` | `SIGSEGV` (signal 11) | `err_e25_records_null_ptr_deref` (subprocess) | [x] |
| E26 | `manipulate_records` | `num_records == 1, shift == 1` — boundary one past the largest accepted `shift` | returns `0` | `err_e26_records_boundary_one` | [x] |
| E27 | `apply_operation` | `op == NULL` — the callee is called with no validation (`return op(a,b,c)`) | `SIGSEGV` (signal 11) | `err_e27_apply_operation_null_fnptr` (subprocess) | [x] |
| E28 | `apply_operation` | `op` = a bogus, non-executable address (e.g. `0x1`) — "out of range" function pointer, the FFI analogue of an out-of-range enum | `SIGSEGV` (signal 11) | `err_e28_apply_operation_bogus_fnptr` (subprocess) | [x] |
| E29 | `apply_operation` | `op` = a *valid* callee from the **other** library (C's `add_three` passed to Rust's `apply_operation` and vice-versa) — the only "enum-like" domain this API has: any code address is accepted | identical result from both, and the cross-library call works | `err_e29_apply_operation_cross_library` | [x] |
| E30 | `add_three` / `multiply_add` / `complex_calc` | operands at `INT_MIN`/`INT_MAX` so the `+`/`*`/`-` overflow `int` (UB, wraps at `-O0`) | wrapped 32-bit result | `err_e30_arith_overflow_extremes` | [x] |
| E31 | `increment_counter` | `value` making `global_counter += value` overflow `int` (UB, wraps) | `global_counter` wraps; observable through `complex_calc`/`hatch` | `err_e31_increment_counter_overflow` | [x] |
| E32 | `update_accumulator` | `value` making `global_accumulator * 2 + value` overflow `int` (UB, wraps); also the `*2` alone overflows once the accumulator exceeds `INT_MAX/2` | `global_accumulator` wraps; observable through `process_pointer_data`/`hatch` | `err_e32_update_accumulator_overflow` | [x] |
| E33 | `hatch` | all four params at `INT_MIN`/`INT_MAX`/`0`/`-1` extremes — every internal `+`/`*`/`-` overflows | one specific wrapped `int`; must match bit-for-bit | `err_e33_hatch_extremes` | [x] |
| E34 | *whole library* | there is **no** `errno`, no sentinel, no error enum: no function can report failure. Documented as a negative result so it is not mistaken for missing coverage. | n/a | `err_e34_no_error_protocol_exists` | [x] |
| E35 | `shift_array_data` | **oversized length**: guard true with `size == INT_MAX, shift_by == 1` → `memmove` of `(INT_MAX-1)*4` ≈ 8 GiB out of a small buffer | `SIGSEGV` (signal 11) | `err_e35_shift_array_oversized_size` (subprocess) | [x] |
| E36 | `manipulate_records` | **oversized length**: guard true with `num_records == INT_MAX, shift == 1` → `memmove` of `(INT_MAX-1)*48` ≈ 96 GiB | `SIGSEGV` (signal 11) | `err_e36_records_oversized_num` (subprocess) | [x] |
| E37 | `compute_with_dynamic_memory` | **large but valid length**: `count` up to `1<<20` — the `malloc` path that actually succeeds, with a `sum` that wraps many times | wrapped 32-bit sum | `err_e37_cwdm_large_but_valid_count` | [x] |
| E38 | *whole library, dev profile* | Rust's `debug-assertions` inject a `null pointer dereference occurred` panic into the raw-pointer loads, turning the C's `SIGSEGV` into a panic-abort (`SIGABRT`). Found by the build-profile axis of `run_all_features.sh`. **Fixed** by `[profile.dev] debug-assertions = false` in `Cargo.toml`. | `SIGSEGV` in every profile | `err_e9_*`, `err_e25_*`, `err_e27_*`, `err_e8_*` under `run_all_features.sh` profile axis | [x] |
| E39 | `process_pointer_data`, `shift_array_data` | **misaligned** `int *` (offsets 1..7 into a byte buffer) — the C declares `int *` and never checks alignment; UB in C, but an ordinary unaligned load/store on x86-64 | same value / same bytes, no trap | `err_e39_misaligned_int_pointer` | [x] |
| E40 | `manipulate_records` | **misaligned** `DataRecord *` (offsets 1..7) — misaligned `memmove` destination and misaligned `.value` load | same sum and same post-`memmove` byte image, no trap | `err_e40_misaligned_record_pointer` | [x] |

## Notes on the "out-of-range enum value" requirement

`lib.c` declares no `enum` and no flag/mode parameter, so there is no integer
whose domain is a small set of named variants. The structurally equivalent
"value with no valid variant crossing the FFI boundary" for this API is the
**function pointer** argument of `apply_operation` (rows E27/E28/E29) and the
guard-selecting `int`s of `shift_array_data` / `manipulate_records` (rows
E1–E6, E18–E23, E26), which are all covered above, including one step past each
accepted range (`shift_by == size`, `shift == num_records`) and both `int`
extremes.
