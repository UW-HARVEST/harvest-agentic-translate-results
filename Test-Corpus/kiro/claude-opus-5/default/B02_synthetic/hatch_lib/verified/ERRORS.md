# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/lib.c`. Grepping for the usual error
constructs finds **nothing**:

```sh
grep -n 'assert\|return -1\|return NULL\|RETURN_ERROR\|errno\|exit(' c_src/src/lib.c   # no matches
```

This library has **no error codes, no sentinels, no asserts, and no NULL
returns**. Every function returns `int` or `void` and any `int` bit pattern is a
legal result. Its rejection surface is therefore made of:

* **guard conditions** — `if (...)` predicates whose false branch silently skips
  work (the C code's only form of "reject this input"),
* **loop bounds that go non-positive** — the computed count that makes a body
  execute zero times (an implicit rejection),
* **unchecked results / unchecked pointers** — `malloc` returns and pointer
  parameters the C never validates, so an invalid input is a hard fault,
* **implicit conversions on size arguments** — negative `int` widened to
  `size_t` for `malloc`/`memmove`/`memset`,
* **signed-overflow wraparound** — the observable result for out-of-range
  arithmetic.

"Expected C result" below is what the compiled C `.so` actually does, and is
what the Rust `.so` must reproduce bit-for-bit.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `shift_array_data` (lib.c:67 guard, first conjunct) | `shift_by == 0` | guard false → no `memmove`, no `memset`; array left completely unmodified; `void` |
| 2 | `shift_array_data` (lib.c:67 guard, first conjunct) | `shift_by < 0` (e.g. `-1`, `INT_MIN`) | guard false → array unmodified (no negative-offset `memmove`); `void` |
| 3 | `shift_array_data` (lib.c:67 guard, second conjunct) | `shift_by == size` | guard false → array unmodified; `void` |
| 4 | `shift_array_data` (lib.c:67 guard, second conjunct) | `shift_by > size` | guard false → array unmodified; `void` |
| 5 | `shift_array_data` (lib.c:67, both conjuncts) | `size <= 0` with any `shift_by` (incl. `size == 0`, `size < 0`) | guard false (no `shift_by` satisfies `0 < shift_by < size`) → array unmodified; `void` |
| 6 | `shift_array_data` (lib.c:67) | `size == INT_MIN`, `shift_by == INT_MIN` | guard false → array unmodified; `void` |
| 7 | `process_pointer_data` (lib.c:74, unchecked deref) | `ptr == NULL` | no NULL check → dereference of address 0 → process dies on `SIGSEGV` |
| 8 | `compute_with_dynamic_memory` (lib.c:79 unchecked `malloc`, lib.c:81/86 loop bounds) | `count == 0` | `malloc(0)` (result unchecked, never dereferenced), both loops run 0 times, `free`, returns `0` |
| 9 | `compute_with_dynamic_memory` (lib.c:79, `count * sizeof(int)` with negative `int` → `size_t`) | `count < 0` (e.g. `-1`, `INT_MIN`) | `count` sign-extends to a huge `size_t` → `malloc` fails, returns `NULL`; loop guards `i < count` are immediately false so `NULL` is never dereferenced; `free(NULL)` is a no-op; returns `0` |
| 10 | `compute_with_dynamic_memory` (lib.c:82/87 signed overflow) | `count > 0` with `base` near `INT_MAX`/`INT_MIN` so `base + i*3` and/or `sum +=` overflow | wraps modulo 2^32 (two's complement); returns the wrapped `int` |
| 11 | `manipulate_records` (lib.c:111 guard, first conjunct) | `shift == 0` | guard false → no `memmove`; loop bound `num_records - 0` → sums all `num_records` elements |
| 12 | `manipulate_records` (lib.c:111 guard first conjunct + lib.c:116 loop bound) | `shift < 0` | guard false → no `memmove`, **but** the loop bound `num_records - shift` is *larger* than `num_records`, so the loop reads `-shift` elements **past the end** of the caller's array (out-of-bounds read; no rejection at all) |
| 13 | `manipulate_records` (lib.c:111 guard, second conjunct + lib.c:116) | `shift == num_records` | guard false; loop bound `num_records - shift == 0` → loop skipped → returns `0` |
| 14 | `manipulate_records` (lib.c:111 guard second conjunct + lib.c:116) | `shift > num_records` | guard false; loop bound negative → loop skipped → returns `0` |
| 15 | `manipulate_records` (lib.c:111/116) | `num_records == 0` with `shift == 0` | guard false; loop bound `0` → returns `0` |
| 16 | `manipulate_records` (lib.c:111/116) | `num_records < 0` with `shift >= 0` | guard false; loop bound negative → returns `0` |
| 17 | `manipulate_records` (lib.c:116 signed overflow of the bound) | `num_records == INT_MAX`, `shift == INT_MIN` (or `num_records - shift` overflows) | `num_records - shift` wraps to `-1` → loop skipped → returns `0` |
| 18 | `manipulate_records` (lib.c:117 signed overflow of accumulator) | element `.value`s summing past `INT_MAX` | `total` wraps modulo 2^32; returns wrapped `int` |
| 19 | `apply_operation` (lib.c:44, unchecked function pointer) | `op == NULL` | no NULL check → indirect call through address 0 → process dies on a fatal signal |
| 20 | `add_three` (lib.c:48 signed overflow) | `a + b + c` out of `int` range (e.g. all three `INT_MAX`) | wraps modulo 2^32 |
| 21 | `multiply_add` (lib.c:52 signed overflow) | `a * b` and/or `+ c` out of `int` range (e.g. `INT_MIN * -1`) | wraps modulo 2^32 |
| 22 | `complex_calc` (lib.c:56 signed overflow) | `(a - b) * c + global_counter` out of `int` range | wraps modulo 2^32 |
| 23 | `increment_counter` (lib.c:35 signed overflow of the `static`) | repeated calls / `value` driving `global_counter` past `INT_MAX` | `global_counter` wraps modulo 2^32; observable through `complex_calc` and `hatch` |
| 24 | `update_accumulator` (lib.c:39 signed overflow of the `static`) | `global_accumulator * 2 + value` out of `int` range (reached after ~31 calls, since it doubles) | `global_accumulator` wraps modulo 2^32; observable through `process_pointer_data` and `hatch` |
| 25 | `process_pointer_data` (lib.c:75 signed overflow) | `*ptr * multiplier + global_accumulator` out of `int` range | wraps modulo 2^32 |
| 26 | `get_time_based_value` (lib.c:135 signed overflow of `seed * 3600`) | `abs(seed) > INT_MAX/3600` (≈ 596523), e.g. `1000000`, `INT_MAX`, `INT_MIN` | `seed * 3600` wraps modulo 2^32 **before** widening to `time_t`, so `difftime` sees the wrapped value; result is `(int)(wrapped/100.0) + seed`, itself wrapping |
| 27 | `get_time_based_value` (lib.c:139 float→int conversion) | any `seed` (the `(int)(diff / 100)` truncation) | truncation toward zero of the quotient; `diff/100` never exceeds `int` range because `diff` is the wrapped 32-bit value, so no out-of-range float→int conversion occurs |
| 28 | `hatch` (lib.c:145 / lib.c:159 unchecked `malloc`) | allocation failure (fixed 40-byte and 240-byte requests) | unchecked → would dereference `NULL`; not reachable with normal inputs, so `hatch` never rejects any `int` quadruple |
| 29 | `hatch` (lib.c:126 onward, signed overflow throughout) | any params near `INT_MIN`/`INT_MAX` (`0`, `INT_MAX`, `INT_MIN`, mixed) | every accumulation wraps modulo 2^32; `hatch` returns a wrapped `int` and never signals an error |
| 30 | all functions (C ABI, no enum parameters) | out-of-range enum value passed across FFI | **not applicable / vacuous:** this API declares no `enum`, `bool`, or pointer-to-enum parameter — every parameter is `int`, `int*`, `DataRecord*` or a function pointer, so every 32-bit pattern is already an in-range value. Covered by exhaustively fuzzing the full `i32` domain (incl. `INT_MIN`/`INT_MAX`) in rows 20–29 rather than by a separate variant test. |

## Test mapping

Rows 1–6, 8–18, 20–29 are covered by in-process differential tests in
`translation/tests/errors.rs` (`err01_…`–`err30_…`), which call both `.so`s and
assert identical returns and identical post-state buffers.

Rows 7 and 19 are fatal-signal cases. They are covered by
`translation/tests/crash_parity.rs`, which re-executes the test binary in a child
process for each library and asserts **both** die with the **same** signal — not
merely that "both failed somehow". Verified signal: `SIGSEGV` (139) for all
cases, in both the release and debug Rust profiles. The same file also covers two
adjacent fatal cases that fall out of rows 12/16 and 1–6:

* `manipulate_records(NULL, 4, 0)` — guard rejects, loop bound is still 4, so the
  C dereferences NULL;
* `shift_array_data(NULL, 8, 3)` — guard *passes*, so `memmove`/`memset` run on
  NULL inside glibc;

plus `null_pointers_that_the_guards_make_survivable`, which asserts the NULL
cases the guards make harmless return identically and do **not** fault.

Row 28's allocation-failure path is unreachable for the fixed 40/240-byte
requests and is documented as such; it is not testable without an allocator
fault injector, and both implementations make the identical unchecked request.

## Divergence found and fixed

Row 7 initially FAILED when the Rust `.so` was built with UB checks on (the
`dev`/`debug` profile):

| | C `.so` | Rust `.so` (debug, before fix) |
|---|---|---|
| `process_pointer_data(NULL, 3)` | `SIGSEGV` (11) | `SIGABRT` (6), after `panicked at src/lib.rs:176: null pointer dereference occurred` |

Cause: rustc injects a null-dereference assertion for a `*ptr` place expression
whenever `-C debug-assertions` is on, which converts the C's hardware fault into
a Rust panic and a different termination signal. Fix: the translation now reads
and writes through `core::ptr::read` / `core::ptr::write` (and
`core::ptr::read(&raw const (*p).field)` for the `DataRecord` field access),
which compile to the same single load/store but carry no injected check — so the
library faults exactly like the C in **every** profile. Verified: all eight crash
children now report `SIGSEGV` (139) under both the release and debug Rust builds.

The same substitution was applied to the malloc-backed accesses in
`compute_with_dynamic_memory` and `hatch` so an allocation failure there would
also fault identically rather than panicking.

