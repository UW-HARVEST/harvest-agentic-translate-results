# ERRORS.md — Phase A error-surface table

Derived mechanically from the C source, not from docs. The complete set of
conditional / rejecting constructs in `c_src/src/driver.c`:

```
$ grep -n 'return|assert|NULL|RETURN_ERROR|errno|exit|abort|if (|switch|#if|enum|MAX|MIN' src/driver.c
src/driver.c:30:    if (line != NULL)
src/driver.c:51:    if (useGood)
```

That is the entire branch surface: **two `if` statements**. The library has:

* no `return` statements that yield a value (all four functions are `void`),
* no error codes, sentinels, enums, or `RETURN_ERROR`-style macros,
* no `assert`, `abort`, `exit`, or `errno` use,
* no range checks and no min/max constants,
* no `#ifdef` configuration branches.

So the only *rejection* the library performs is `printLine`'s NULL guard. Rows
1–2 below are the real, source-derived rejection surface. Rows 3–9 are the
generic FFI boundary conditions mandated for every C API (null pointers, zero
and oversized lengths, values one step past a valid range, and out-of-range
"enum"/int values crossing the FFI boundary). `driver`'s `int useGood` is the
only parameter with a value domain, and because the C tests it with a bare
truthiness check, *every* non-zero `int` is valid input — there is no
out-of-range value that the C rejects. That is recorded faithfully below rather
than invented as an error.

"Expected C result" is the observable behaviour: what reaches stdout, and the
return value (always none — `void`).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | NULL guard at `driver.c:30` fails; `puts` is **not** called; nothing written to stdout; returns normally (`void`). No crash. |
| 2 | `bad` → `printLine` | the uninitialized `char *data` happens to hold `NULL` (residue-dependent) | same as row 1: nothing written, returns normally. Reached whenever the `-0x8(%rbp)` slot residue is zero. |
| 3 | `printLine` | `line` points to an empty string `""` (zero-length, not NULL) | passes the NULL guard; `puts("")` writes a single `\n` (1 byte). Distinct from row 1, which writes 0 bytes. |
| 4 | `printLine` | `line` points to a 1-byte buffer holding only `'\0'` | identical to row 3: one `\n`. |
| 5 | `printLine` | oversized input: `line` points to a very long string (e.g. 1 MiB, no interior NUL) | no length check exists; `puts` writes all bytes then `\n`. No truncation, no rejection. |
| 6 | `printLine` | `line` points at a buffer whose first byte is `'\0'` but which has trailing bytes after it | `puts` stops at the first NUL: writes only `\n`. Trailing bytes are not emitted. |
| 7 | `driver` | `useGood == 0` (the false branch, `driver.c:51`) | calls `bad()`, i.e. the CWE-457 path: forwards the uninitialized pointer to `printLine`. Output is stack-residue dependent. |
| 8 | `driver` | `useGood` one step past / outside any "expected" 0/1 domain: `-1`, `2`, `INT_MIN` (`-2147483648`), `INT_MAX` (`2147483647`) | **not an error.** `if (useGood)` is a bare truthiness test, so every non-zero value takes the `good()` branch and prints `string\n`. No validation, no rejection. Must be replicated exactly — the Rust must not range-check. |
| 9 | `driver` | out-of-range "enum-like" int passed across the FFI boundary (values with no meaningful variant, e.g. `0x80000000` truncated to `int`, `3`, `999999`) | as row 8: any non-zero → `good()` → `string\n`; only exact zero → `bad()`. The parameter is a plain `int`, so all 2^32 bit patterns are accepted inputs. |

## Notes on what is *not* an error path

`bad()` (CWE-457, use of an uninitialized pointer) is **undefined behaviour, not
a rejection**. The C does not detect or report it; it forwards whatever
pointer-sized residue occupies its `-0x8(%rbp)` stack slot. Its observable
output is therefore a property of the *stack state at the call site*, not of the
argument. It is covered as a configuration axis in `CONFIGS.md` (Phase B) with
the call-site stack state controlled, plus rows 2 and 7 here.

Consequently there is no input for which the C returns an error code and the
Rust could return a different one: the error surface is exactly "print nothing"
vs "print something". Every row above asserts byte-equal stdout **and**
equal (normal, non-crashing) completion of the call across the FFI boundary.

## Results — every row has a passing differential test

Tests are in `tests/phase_c_errors.rs`; all 15 pass. Each loads both `.so`s via
`libloading` and calls only their exported symbols.

| # | test | outcome |
|---|------|---------|
| 1 | `err01_printline_null_rejects_lazy`, `err01_printline_null_emits_nothing` | PASS — both sides emit **exactly 0 bytes**, pinned absolutely, not merely equal to each other. Covered cold, repeated (`pn,pn,pn`) and interleaved with accepted input, under LAZY and NOW. |
| 2 | `err02_bad_null_residue` | PASS — the decisive form primes the slot with NULL from the immediately preceding library call (`tn`), proving `bad()` reaches the guard: both sides emit 0 bytes. Unprimed forms are compared modulo loader-residue lines (see `CONFIGS.md` → *Attribution*), which still pins both sides to the same branch count. |
| 3 | `err03_printline_empty_string_emits_newline` | PASS — exactly `\n` (1 byte) on both, pinned; distinct from row 1's 0 bytes. |
| 4 | `err04_printline_lone_nul_byte` | PASS — identical to row 3. |
| 5 | `err05_printline_oversized_no_truncation` | PASS — 4 KiB, 64 KiB and 1 MiB inputs; output length pinned to `n + 1`, so absence of truncation and of any length check is asserted, not assumed. |
| 6 | `err06_printline_nul_first_with_trailing_bytes` | PASS — `"\0abc"` yields exactly `\n`; trailing bytes are not emitted. |
| 7 | `err07_driver_zero_takes_bad_branch` | PASS — LAZY and NOW. |
| 8 | `err08_driver_out_of_domain_values_are_not_rejected` | PASS — `-1`, `2`, `3`, `999999`, `-999999`, `INT_MIN`, `INT_MAX` each pinned to `string\n`, confirming the Rust adds no range check. |
| 9 | `err09_driver_out_of_range_enum_like_ints_across_ffi` | PASS — all 32 single-bit patterns (including the sign bit) plus 64 randomized full-width `int`s, under LAZY and NOW. |

Generic boundaries beyond the table, also passing:

* `generic_null_then_every_entry_point` — a NULL-rejecting call before each other
  entry point, LAZY and NOW.
* `generic_high_bytes_and_non_utf8` — `0xFF`, truncated UTF-8 lead bytes,
  invalid sequences, an encoded surrogate half, a value above `U+10FFFF`, lone
  continuation bytes, and one buffer containing all 255 non-NUL bytes. A
  translation that assumed UTF-8 would diverge or panic here; this one does not.
* `generic_zero_and_oversized_lengths_together` — lengths 0, 1, 2, 1023, 1024,
  1025, 65535, 65536 in one process.
* `generic_no_crash_on_any_row` — a long program touching every entry point and
  both branches; both children must exit 0, so a segfault or abort on either
  side fails the test.

No divergence was found on any error path, so no fix to the Rust was required
for Phase C. The suite's ability to *detect* an error-path defect is verified
independently: inverting `printLine`'s NULL guard in a C build fails 37 tests
(`scripts/mutation_check.sh`).
