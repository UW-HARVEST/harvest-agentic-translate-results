# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived **mechanically** from `c_src/src/driver.c` + `c_src/include/driver.h`
(the complete C source: 37 + 28 lines). Raw greps performed over the whole C
tree:

```sh
grep -nE 'RETURN_ERROR|return[^;]*;|assert|NULL|nullptr|errno|goto|exit\(|abort\(' c_src/src/driver.c c_src/include/driver.h
#   -> (no matches other than the #endif comment in the header)
grep -nE '\bif\b|\bswitch\b|\?|&&|\|\|' c_src/src/driver.c
#   -> (no matches: the only control flow is the `for` loop)
grep -nE 'enum|#define|INT_MAX|INT_MIN|SIZE_MAX|MAX|MIN|<=|>=|!=' c_src/src/driver.c
#   -> (no matches)
```

## Mechanical findings

| grep target | occurrences in C |
|-------------|------------------|
| error-return macro (`RETURN_ERROR`, …) | 0 |
| `return <error value>` (`-1`, `NULL`, enum, …) | 0 (both functions are `void`; no `return` statement at all) |
| `assert` / `abort` / `exit` | 0 |
| explicit range check (`if (x < …)`, `if (n > MAX)`) | 0 |
| null-pointer check | 0 |
| min/max constant, magic limit, `#define` | 0 |
| `enum` type in the public header | 0 |
| pointer parameter in the public API | 0 (`void driver(int x)`) |
| `errno` inspection | 0 |
| return value of `printf` checked | 0 — **ignored at both call sites** (lines 30, 32) |
| conditional constructs of any kind | 1: the loop condition `i < len` (line 29) |

**Consequence:** `driver` has **no rejection path and no error channel**. It is
`void`, validates nothing, and cannot fail-report. Every 32-bit input value is
accepted. The "error surface" therefore consists of (a) the single implicit
condition in the code (`i < len`), (b) the silently-ignored `printf` failures,
and (c) the generic FFI boundaries that exist for any C API. All of these are
enumerated below as rows and all are differentially tested.

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `print_hex` (internal) | `len == 0` — loop guard `i < len` false on entry | loop body skipped; only `"\n"` (1 byte) printed; returns `void` | `err_e1_len_zero_and_negative_are_unreachable_but_shape_is_fixed` | [x] |
| E2 | `print_hex` (internal) | `len < 0` (e.g. `-1`) — loop guard false on entry, no negative indexing | loop body skipped; only `"\n"` printed; no read of `p`; returns `void` | `err_e1_len_zero_and_negative_are_unreachable_but_shape_is_fixed` | [x] |
| E3 | `driver` | *any* `int` value, incl. the extremes `INT_MIN` / `INT_MAX` / `-1` / `0` | **not rejected** — no error is possible; always emits exactly `2*sizeof(int)+1 == 9` bytes; returns `void` | `err_e3_no_value_is_ever_rejected` | [x] |
| E4 | `driver` → `printf` | `stdout` write fails with `EBADF` (fd 1 points at a descriptor opened read-only, so every `write(2)` returns `EBADF`) | `printf`'s negative return value is **ignored**; `driver` returns normally; no crash, no message, no `errno` propagation | `err_e4_stdout_ebadf` | [x] |
| E5 | `driver` → `printf` | `stdout` write fails with `ENOSPC` (fd 1 redirected to `/dev/full`) | return value ignored; `driver` returns normally; no crash | `err_e5_stdout_enospc_dev_full` | [x] |
| E6 | `driver` → `printf` | `stdout` write fails with `EPIPE` (fd 1 is a pipe whose read end is closed) | return value ignored; `driver` returns normally (`SIGPIPE` disposition is process-wide and identical for both libraries); no crash | `err_e6_stdout_epipe` | [x] |
| E7 | `driver` → `printf` | `stdout` is *already* in an error state (sticky `FILE` error flag from a previous failed flush) | return value ignored; `driver` returns normally; nothing is emitted | `err_e7_sticky_stream_error_state` | [x] |
| E8 | `print_hex` | symbol requested through `dlsym` (it is `static`, so it must be unreachable) | `dlsym` fails / symbol not found | `err_e8_internal_symbol_not_exported` | [x] |

## Generic C-API boundaries (mandated coverage, even though absent from the table above)

| # | boundary | applicability to this API | expected C result | test | status |
|---|----------|---------------------------|-------------------|------|--------|
| G1 | null pointer argument | **N/A** — `driver` has no pointer parameter (`void driver(int)`); verified by grep: 0 pointer params in `driver.h` | — (documented, nothing to call) | `err_g1_no_pointer_parameters_in_public_api` (asserts the header shape) | [x] |
| G2 | zero length / oversized length argument | **N/A** — `driver` has no length parameter; the only length is the compile-time constant `sizeof(x)`. The internal length boundaries are rows E1/E2. | fixed 9-byte output for every input | `err_e1_len_zero_and_negative_are_unreachable_but_shape_is_fixed` | [x] |
| G3 | value one step past a documented valid range | the documented range of `int` is the whole 32-bit domain; the "one step past" cases are `INT_MIN`, `INT_MAX`, `INT_MIN-1` and `INT_MAX+1` **as they wrap when marshalled across FFI** | wrapped 32-bit value printed; never an error | `err_g3_one_past_int_range_wraps` | [x] |
| G4 | out-of-range enum value across the FFI boundary | **N/A by type** — the public header declares no `enum`. The equivalent "bit pattern with no valid variant" for this API is an `int` argument whose upper 32 register bits carry garbage (a 64-bit caller value): C must read only the low 32 bits. | only the low 32 bits are printed; upper bits ignored | `err_g4_upper_register_bits_ignored` | [x] |
| G5 | repeated / re-entrant invocation after an error | calling `driver` again after E4/E5/E6 | works exactly as before for whichever stream state remains; no persistent library state exists (the C has **no** globals — grep: 0 file-scope variables) | `err_g5_call_after_error_recovers_identically` | [x] |

All rows E1–E8 and G1–G5 are covered by `tests/error_paths.rs`.

## Phase C result

```
running 12 tests
test err_e1_len_zero_and_negative_are_unreachable_but_shape_is_fixed ... ok
test err_e3_no_value_is_ever_rejected ... ok
test err_e4_stdout_ebadf ... ok
test err_e5_stdout_enospc_dev_full ... ok
test err_e6_stdout_epipe ... ok
test err_e7_sticky_stream_error_state ... ok
test err_e8_internal_symbol_not_exported ... ok
test err_g1_no_pointer_parameters_in_public_api ... ok
test err_g3_one_past_int_range_wraps ... ok
test err_g4_upper_register_bits_ignored ... ok
test err_g5_call_after_error_recovers_identically ... ok
test err_randomized_failure_states_match ... ok

test result: ok. 12 passed; 0 failed
```

Rows E1/E2 share one test (they are the two halves of the same `i < len` guard);
row G2 is covered by that same test, as noted in its row. Every row is checked.

The failure-state comparisons assert the *specific* outcome, not merely "both
failed": `ferror(stdout) != 0` **and** `errno == EBADF` (E4) / `ENOSPC` (E5) /
`EPIPE` (E6), identical for C and Rust. E4/E5 are additionally repeated over 64
randomized inputs in `err_randomized_failure_states_match`.
