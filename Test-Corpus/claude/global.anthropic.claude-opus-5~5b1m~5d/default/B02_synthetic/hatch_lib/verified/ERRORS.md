# ERRORS.md — Phase C error/rejection surface table

Mechanically derived from `grep -n 'return\|assert\|NULL\|if (\|#if\|MAX\|MIN\|errno' c_src/src/lib.c`.

**Finding:** the C library has **no error codes, no sentinels, no `assert`, no null
checks, no `errno` use, and no range constants.** Every `return` is a value return.
The complete rejection surface therefore consists of:

* two *guard conditions* that silently skip work (`shift_array_data`, `manipulate_records`),
* several *unchecked* operations that are the C's actual observable behaviour on
  invalid input (unchecked `malloc`, unchecked pointer deref, negative counts).

Rows below are one per distinct rejecting/guarding branch or unchecked-input path.
"expected C result" is what the C actually does, which is what Rust must reproduce.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `shift_array_data` | `shift_by == 0` (fails `shift_by > 0`) | no-op: array unchanged, no memmove/memset | [x] |
| 2  | `shift_array_data` | `shift_by < 0` (fails `shift_by > 0`) | no-op: array unchanged | [x] |
| 3  | `shift_array_data` | `shift_by == size` (fails `shift_by < size`) | no-op: array unchanged | [x] |
| 4  | `shift_array_data` | `shift_by > size` (fails `shift_by < size`) | no-op: array unchanged | [x] |
| 5  | `shift_array_data` | `size <= 0` with any `shift_by` (fails `shift_by < size`) | no-op: array unchanged | [x] |
| 6  | `shift_array_data` | `shift_by == size - 1` (last accepted value) | memmove 1 elem, memset `size-1` elems to 0 | [x] |
| 7  | `shift_array_data` | `shift_by == 1` (first accepted value) | full shift-by-one, last elem zeroed | [x] |
| 8  | `manipulate_records` | `shift == 0` (fails `shift > 0`) | no memmove; sums `records[0..num_records)` | [x] |
| 9  | `manipulate_records` | `shift < 0` (fails `shift > 0`) | no memmove; loop bound `num_records - shift` > `num_records` ⇒ **reads past `num_records`** and sums those bytes | [x] |
| 10 | `manipulate_records` | `shift == num_records` (fails `shift < num_records`) | no memmove; loop bound 0 ⇒ returns `0` | [x] |
| 11 | `manipulate_records` | `shift > num_records` (fails `shift < num_records`) | no memmove; loop bound < 0 ⇒ returns `0` | [x] |
| 12 | `manipulate_records` | `num_records == 0`, `shift == 0` | returns `0` | [x] |
| 13 | `manipulate_records` | `num_records < 0` (any `shift >= num_records`) | loop bound <= 0 ⇒ returns `0`, no memmove | [x] |
| 14 | `manipulate_records` | `num_records < 0` and `0 < shift < num_records` — unreachable (no `shift` satisfies both) | guard never taken | [x] |
| 15 | `compute_with_dynamic_memory` | `count == 0` | `malloc(0)`; both loops skipped; returns `0` | [x] |
| 16 | `compute_with_dynamic_memory` | `count < 0` | `count * sizeof(int)` sign-extends to a huge `size_t` ⇒ `malloc` returns NULL (unchecked), loops skipped (`i < count` false), `free(NULL)`, returns `0` | [x] |
| 17 | `compute_with_dynamic_memory` | `count == 1` (boundary above 0) | returns `base` | [x] |
| 18 | `compute_with_dynamic_memory` | `base` = `INT_MAX` / `INT_MIN` ⇒ signed overflow in `base + i*3` and in `sum +=` | wraps two's-complement (gcc codegen) | [x] |
| 19 | `get_time_based_value` | `seed` large enough that `seed * 3600` overflows `int` (e.g. `seed > 596523`) | wraps in `int`, then sign-extends into `time_t`; `(int)(diff/100) + seed` | [x] |
| 20 | `get_time_based_value` | `seed == INT_MIN` / `INT_MAX` | as row 19, plus wrapping `+ seed` | [x] |
| 21 | `add_three` / `multiply_add` / `complex_calc` | operands at `INT_MIN`/`INT_MAX` ⇒ signed overflow | two's-complement wrap | [x] |
| 22 | `increment_counter` / `update_accumulator` | value driving `global_counter` / `global_accumulator` past `INT_MAX` | two's-complement wrap of the global | [x] |
| 23 | `hatch` | any `param1..param4` at `INT_MIN`/`INT_MAX` (overflow in every arithmetic step, incl. the persistent globals) | single wrapped `int` | [x] |
| 24 | `apply_operation` | `op == NULL` | call through null pointer ⇒ SIGSEGV, *no* error return | [x] (subprocess) |
| 25 | `process_pointer_data` | `ptr == NULL` | `*ptr` deref of NULL ⇒ SIGSEGV, *no* error return | [x] (subprocess) |
| 26 | `shift_array_data` | `arr == NULL` with a *passing* guard (`0 < shift_by < size`) | `memmove` from NULL ⇒ SIGSEGV | [x] (subprocess) |
| 27 | `shift_array_data` | `arr == NULL` with a *failing* guard (e.g. `shift_by = 0`) | no-op, **no** crash (guard short-circuits before any deref) | [x] |
| 28 | `manipulate_records` | `records == NULL`, `num_records <= 0`, `shift >= num_records` | no-op, returns `0`, **no** crash | [x] |
| 29 | enum-shaped input | the C API declares **no enums** (`c_src/include/lib.h` exports only `int hatch(int,int,int,int)`) — every parameter is a plain `int`, so "out-of-range enum value" degenerates to arbitrary `int`, covered by rows 18–23 | n/a | [x] |
| 30 | oversized length | `shift_array_data(arr, INT_MAX, INT_MAX-1)` — guard passes, `memmove` of ~4 GiB from a small buffer | SIGSEGV (both) | [x] (subprocess) |

Rows 24–26 and 30 are verified by re-exec'ing the test binary as a child process and
asserting **both** C and Rust die with the *same* signal, since a matching crash is
the C's actual behaviour and cannot be observed in-process.
