# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/driver.c`. Every rejection / guard /
error-ish construct in the C source is one row. The grep basis:

```
grep -n 'return\|NULL\|assert\|if\|else\|<\|>\|==\|!=' c_src/src/driver.c
```

The library has **no** error enum, **no** `RETURN_ERROR` macro, **no**
`assert`, and **no** function that returns a value — every public function
returns `void`. Therefore the entire "error surface" consists of the single
explicit guard in `printLine` plus the language-level boundary conditions
that any C API of this shape has. All are enumerated below.

## Explicit guards in the C source

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` (`if (line != NULL)` at driver.c:30 is false) | returns silently; **zero bytes** written to stdout; no crash | [x] |

## Implicit / language-level boundaries (must still be covered)

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 2 | `printLine` | `line` points at an empty string `""` (length-0, not NULL) | prints exactly `"\n"` (guard passes, `puts("")`) | [x] |
| 3 | `printLine` | `line` points at a non-NUL-terminated-looking buffer whose 1st byte is `\0` | prints exactly `"\n"` | [x] |
| 4 | `printLine` | `line` is a *misaligned* / arbitrary non-null pointer to valid NUL-terminated bytes (e.g. `buf + 1`) | prints those bytes + `"\n"` | [x] |
| 5 | `printLine` | `line` is a huge (oversized, 64 KiB) NUL-terminated buffer | prints all bytes + `"\n"`, no truncation | [x] |
| 6 | `printLine` | `line` contains bytes that look like `printf` directives (`%s`, `%n`, `%%`) | printed **verbatim** — it is the argument, not the format | [x] |
| 7 | `printLine` | `line` contains non-ASCII / high bytes `0x80..0xFF` | printed verbatim byte-for-byte + `"\n"` | [x] |
| 8 | `driver` | `useGood == 0` (the false branch, `if (useGood)`) | calls `bad()` — reads an **uninitialized** `char *` (UB, see below) | [x] |
| 9 | `driver` | `useGood` = any non-zero int, incl. negative, `INT_MIN`, `INT_MAX`, and values with only high bits set (`0x100` truncation probe) | truthy → calls `good()` → prints `"string\n"`. No range check exists, so **no value is rejected**. | [x] |
| 10 | `driver` | `useGood` is an out-of-range "enum-like" int passed across FFI (e.g. `7`, `-1`, `0x7FFFFFFF`) | C has no enum and no validation: any non-zero behaves as `1`. Must not be rejected. | [x] |
| 11 | `bad` | called with no arguments — unconditional defect: `char *data;` is read while **uninitialized** (driver.c:38-39) | **Undefined behaviour.** The compiled C loads the stale 8 bytes at `-0x8(%rbp)` and passes them to `printLine`, so the output is *caller-stack dependent* and NOT a fixed byte string (observed: `"\n"`, `"\x02\n"`, `"string\n"` depending on call history). Not a rejection; documented as the one input whose result is not byte-reproducible. | [x] |
| 12 | `good` | called with no arguments | always prints `"string\n"`; has no failure mode | [x] |

## Row → test map (all rows covered, all passing)

| row | test in `tests/phase_c_errors.rs` |
|-----|-----------------------------------|
| 1 | `err01_print_line_null_is_rejected_silently`, `err01b_print_line_null_repeated_and_interleaved`, `err_x_generic_boundary_pointers` |
| 2 | `err02_print_line_empty_string_prints_newline` |
| 3 | `err03_print_line_buffer_whose_first_byte_is_nul` |
| 4 | `err04_print_line_misaligned_interior_pointer` |
| 5 | `err05_print_line_oversized_length_not_truncated` |
| 6 | `err06_print_line_format_directives_are_data_not_format` |
| 7 | `err07_print_line_high_bytes_pass_through_verbatim` |
| 8 | `err08_driver_zero_selects_bad_and_never_aborts` |
| 9 | `err09_driver_out_of_range_enumlike_values_are_not_rejected` |
| 10 | `err09_…`, `err10_driver_zero_is_the_only_falsy_value` (exhaustive `-4096..=4096`) |
| 11 | `err11_bad_returns_normally_from_many_call_depths` |
| 12 | `err12_good_has_no_failure_mode` |

## Note on row 11 (the intentional CWE-457 defect) — MEASURED

`bad()` is the only place where C and Rust cannot be held to a byte-for-byte
contract, because the C reads an **uninitialized** stack slot and passes it to
`puts`. This was measured, not assumed:

* The compiled C `bad()` is literally
  `mov -0x8(%rbp),%rax ; mov %rax,%rdi ; call printLine` — it forwards whatever
  8 bytes the previous frame left at that offset.
* Observed C outputs from different callers in one process, in order:
  `"\n"`, `"\x02\n"`, `"string\n"` (the last one being the stale pointer left
  by an earlier `good()` call at the same stack depth).
* Under a fork-per-call probe (200 calls) the C library **SIGSEGV'd in 100 of
  200 runs** — it crashes for the `driver(0)` call shape and survives for the
  direct `bad()` call shape. The Rust library crashed 0/200 times.

There is therefore no defined C result for this row to match. The tests assert
the parts that *are* well defined and check them differentially:

1. `driver(0)` routes to the `bad` branch in **both** libraries — its output is
   never `"string\n"` (which is what all 4·10⁹ non-zero inputs produce).
2. The Rust library must return normally and emit a complete
   (newline-terminated) line at every call depth — never a crash, never a
   partial line.
3. The C's outcome is observed crash-isolated in a forked child (see
   `run_in_child` in `tests/common/mod.rs`) and logged, so its UB cannot take
   down the test runner and cannot be mistaken for a Rust defect.

Every other row is asserted byte-for-byte against the C.
