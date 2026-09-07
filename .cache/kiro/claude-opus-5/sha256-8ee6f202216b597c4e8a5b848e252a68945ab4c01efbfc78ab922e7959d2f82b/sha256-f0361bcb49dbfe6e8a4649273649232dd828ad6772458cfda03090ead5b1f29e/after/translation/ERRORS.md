# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`.

Mechanical grep results over the whole C source:

```
grep -nE "assert|NULL|errno|ERROR|exit\(|abort\(" src/lib.c   -> no matches
grep -nE "#if|switch|case "                        src/lib.c   -> no matches
grep -n  "return"                                  src/lib.c   -> 9 matches, none an error sentinel
```

**There is no error-code / sentinel / `assert` surface in this library at all.**
Every function returns a plain computed `int` or `void`; nothing validates its
arguments and nothing can report failure. The rejection surface is therefore made
up of the paths where the C *silently declines to act*, the *degenerate /
out-of-range* argument values it accepts anyway, the *signed-overflow* paths, and
the *undefined-behaviour* paths (NULL deref, allocation failure, out-of-bounds
read). All of those are real inputs an external caller can pass across the FFI
boundary, and the Rust must reproduce each one bit-for-bit — including the
crashes.

There are **no `enum` types anywhere in the C source or header**, so the
"out-of-range enum value across FFI" class has no instance here. Its nearest
analogue — an out-of-range/`NULL` *function pointer* passed as `operation_func` —
is covered by row 22.

`expected C result` below is what the compiled C `.so` actually does (verified by
the differential tests, not assumed).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `shift_array_data` | `shift_by == 0` (fails `shift_by > 0`) | no-op; array bytes unchanged |
| 2 | `shift_array_data` | `shift_by < 0` (e.g. `-1`, `INT_MIN`) | no-op; array bytes unchanged |
| 3 | `shift_array_data` | `shift_by == size` (fails `shift_by < size`) | no-op; array bytes unchanged |
| 4 | `shift_array_data` | `shift_by > size` | no-op; array bytes unchanged |
| 5 | `shift_array_data` | `size <= 0` with `shift_by > 0` | no-op; array bytes unchanged |
| 6 | `shift_array_data` | `size == INT_MIN`, `shift_by == 1` | no-op (`1 < INT_MIN` false) |
| 7 | `shift_array_data` | `arr == NULL` but guard false (`size==0, shift_by==0`) | returns normally, no dereference |
| 8 | `shift_array_data` | `arr == NULL` and guard true (`size==10, shift_by==3`) | `memmove(NULL,...)` → SIGSEGV |
| 9 | `manipulate_records` | `shift == num_records` | no memmove; loop bound `0` → returns `0` |
| 10 | `manipulate_records` | `shift > num_records` | no memmove; loop bound negative → returns `0` |
| 11 | `manipulate_records` | `shift < 0` (e.g. `-2`, `num_records==5`) | **no** memmove, but loop still runs `num_records - shift == 7` iterations → reads 2 records past the logical end (OOB read); total includes those bytes |
| 12 | `manipulate_records` | `num_records == 0`, `shift == 0` | returns `0` |
| 13 | `manipulate_records` | `num_records < 0`, `shift == 0` | loop bound negative → returns `0` |
| 14 | `manipulate_records` | `num_records - shift` overflows `int` (`num_records==1`, `shift==INT_MIN`) | wraps to negative → no memmove (`INT_MIN > 0` false) → returns `0` |
| 15 | `manipulate_records` | `records == NULL`, `num_records==0`, `shift==0` | returns `0`, no dereference |
| 16 | `manipulate_records` | `records == NULL`, guard true (`num_records==5, shift==2`) | `memmove(NULL,...)` → SIGSEGV |
| 17 | `compute_with_dynamic_memory` | `count == 0` | `malloc(0)`, both loops skipped, `free`, returns `0` |
| 18 | `compute_with_dynamic_memory` | `count < 0` (e.g. `-1`): `count * sizeof(int)` converts to `size_t` and wraps to `0xFFFF_FFFF_FFFF_FFFC` | `malloc` fails → `NULL`; loops skipped (`i < count` false); `free(NULL)`; returns `0` |
| 19 | `compute_with_dynamic_memory` | `count == INT_MIN` | same as 18 → returns `0` |
| 20 | `compute_with_dynamic_memory` | `count` positive but so large `malloc` fails (`count == INT_MAX`) | writes through `NULL` in the fill loop → SIGSEGV |
| 21 | `process_pointer_data` | `ptr == NULL` | `*ptr` → SIGSEGV |
| 22 | `apply_operation` | `op == NULL` (out-of-range function pointer across FFI) | indirect call through `NULL` → SIGSEGV |
| 23 | `get_time_based_value` | `seed * 3600` overflows `int` (`seed == INT_MAX`) | int wrap, then `(int)(diff/100) + seed`, wrapping |
| 24 | `get_time_based_value` | `seed == INT_MIN` | int wrap as above |
| 25 | `add_three` | signed overflow (`INT_MAX, 1, 0`; `INT_MIN, -1, 0`) | two's-complement wrap |
| 26 | `multiply_add` | signed overflow (`INT_MAX, INT_MAX, INT_MAX`) | two's-complement wrap |
| 27 | `complex_calc` | signed overflow in `(a-b)*c` (`INT_MIN, INT_MAX, INT_MAX`) | two's-complement wrap, then `+ global_counter` wrapping |
| 28 | `increment_counter` | accumulated `global_counter` overflow (`INT_MAX` then `+1`) | two's-complement wrap of the static |
| 29 | `update_accumulator` | `global_accumulator * 2` overflow (repeated large values) | two's-complement wrap of the static |
| 30 | `hatch` | all params at `INT_MIN` / `INT_MAX` (overflow through the whole composed pipeline and both statics) | wrapping result; statics left in the wrapped state |

## Checklist

- [x] 1  `shift_array_data` `shift_by == 0`
- [x] 2  `shift_array_data` `shift_by < 0`
- [x] 3  `shift_array_data` `shift_by == size`
- [x] 4  `shift_array_data` `shift_by > size`
- [x] 5  `shift_array_data` `size <= 0`
- [x] 6  `shift_array_data` `size == INT_MIN`
- [x] 7  `shift_array_data` NULL, guard false
- [x] 8  `shift_array_data` NULL, guard true → SIGSEGV
- [x] 9  `manipulate_records` `shift == num_records`
- [x] 10 `manipulate_records` `shift > num_records`
- [x] 11 `manipulate_records` `shift < 0` OOB read
- [x] 12 `manipulate_records` `num_records == 0`
- [x] 13 `manipulate_records` `num_records < 0`
- [x] 14 `manipulate_records` `num_records - shift` overflow
- [x] 15 `manipulate_records` NULL, guard false
- [x] 16 `manipulate_records` NULL, guard true → SIGSEGV
- [x] 17 `compute_with_dynamic_memory` `count == 0`
- [x] 18 `compute_with_dynamic_memory` `count < 0`
- [x] 19 `compute_with_dynamic_memory` `count == INT_MIN`
- [x] 20 `compute_with_dynamic_memory` `count == INT_MAX` → SIGSEGV
- [x] 21 `process_pointer_data` NULL → SIGSEGV
- [x] 22 `apply_operation` NULL op → SIGSEGV
- [x] 23 `get_time_based_value` `INT_MAX`
- [x] 24 `get_time_based_value` `INT_MIN`
- [x] 25 `add_three` overflow
- [x] 26 `multiply_add` overflow
- [x] 27 `complex_calc` overflow
- [x] 28 `increment_counter` static overflow
- [x] 29 `update_accumulator` static overflow
- [x] 30 `hatch` boundary params

## Row → test mapping

Every row has a differential test in `tests/phase_c_errors.rs`, named
`err<NN>_...` matching the row number. All 30 pass; each asserts C and Rust
produce the *same* result — and for the rows where the table states a concrete
value (`0`, "no-op", a specific wrapped sum), the test additionally asserts the C
actually produced it, so a row cannot pass merely because both sides are wrong
in the same way.

Rows 8, 16, 20, 21, 22 are undefined behaviour in C (NULL deref / allocation
failure). They are executed in a `fork()`ed child with `RLIMIT_CORE` at 0, and
compared by *termination status*, so "C faults but Rust returns" or "both fault
but with different signals" is a detected divergence. Row 20 additionally lowers
the child's `RLIMIT_AS` to 512 MB so `malloc` genuinely fails.

Row 14 note: `(num_records, shift) = (INT_MIN, 1)` wraps the loop bound to
`INT_MAX`, which would make the C read out of bounds without end. That case is
not runnable, so the test asserts the guard/bound arithmetic for the runnable
wrapped-bound cases and skips only the unbounded one. `(INT_MIN, INT_MAX)` wraps
to `+1` and does sum one record — the table's blanket "returns 0" was corrected
to the per-case wrapped bound after the C was observed.

## Completion gate

- [x] All 30 rows have a passing differential test (`cargo test --test phase_c_errors`: 30 passed).
- [x] Generic boundaries also covered outside the table: NULL pointers (rows 7, 8,
      15, 16, 21), zero lengths (rows 12, 17), oversized lengths (rows 4, 10, 20),
      one-step-past-range (rows 3, 9, and the exhaustive
      `soak_*_exhaustive_small_grid` sweeps), and an out-of-range function
      pointer across FFI (row 22 — this library declares no `enum`, so a
      pointer with no valid target is the closest real instance).
