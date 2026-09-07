# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/driver.c` (68 lines, the only C source).
Exhaustive grep of every construct that can reject, guard, or error:

```
grep -n 'return'                       driver.c   -> 2 hits (lines 39, 50; both plain value returns, no error path)
grep -n 'NULL'                         driver.c   -> 1 hit  (line 30: if (line != NULL))
grep -n 'assert'                       driver.c   -> 0 hits
grep -n 'RETURN_ERROR\|errno\|exit(\|abort(' driver.c -> 0 hits
grep -n 'if (\|switch\|#if'             driver.c   -> 2 hits (line 30 null guard, line 60 flag branch)
grep -n '<=\|>=\|<\|>'                  driver.c   -> 0 range checks
```

There are **no error codes, no error enums, no sentinels, no asserts, no range
checks and no min/max constants** in this library. Every public function returns
`void`. The *only* rejection the C code performs is the null guard in
`printLine`, whose observable "error result" is: produce no output and return
normally.

Rows below are the complete rejection surface, plus the generic FFI-boundary
boundary cases the task requires (null pointers, zero/oversized lengths,
out-of-range enum-like `int` values) even though the C source does not
explicitly branch on them.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `printLine` | `line == NULL` (the `if (line != NULL)` guard at driver.c:30 fails) | no bytes written to stdout, returns normally, no crash | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at a zero-length string (`""`, i.e. first byte is the NUL terminator) — passes the null guard, so `printf("%s\n","")` runs | writes exactly one byte `"\n"` | `err_e2_print_line_empty` | [x] |
| E3 | `printLine` | `line` contains `printf` conversion specifiers (`"%s %n %p %d %%"`). It is an *argument*, never the format string, so it must NOT be interpreted | the bytes are echoed verbatim + `"\n"`; no format-string evaluation, no crash | `err_e3_print_line_format_specifiers` | [x] |
| E4 | `bad` | inherent defect CWE-562: `helperBad` returns the address of the automatic array `charString` (driver.c:37-39), whose lifetime ended. GCC diagnoses this (`-Wreturn-local-addr`) and materialises `0` for the return value (`mov $0x0,%eax`, verified by `objdump -d` on the built `.so`), so `bad()` calls `printLine(NULL)` and hits row E1 | no bytes written to stdout, returns normally | `err_e4_bad_returns_dangling_is_null` | [x] |
| E5 | `driver` | `useGood == 0` — the falsy branch of `if (useGood)` at driver.c:60, i.e. the *defective* path is selected | dispatches to `bad()`, therefore no output (row E4) | `err_e5_driver_zero_selects_bad` | [x] |
| E6 | `driver` | out-of-range "enum" values: `useGood` given every `int` with no meaningful variant — `-1`, `2`, `3`, `0x100`, `INT_MIN`, `INT_MAX`, and randomized non-zero `i32`s. C `int` accepts any value; the C only tests truthiness | every non-zero value takes the `good()` branch (`"helperGood1 string\n"`); only exact `0` takes `bad()`. No clamping, no rejection, no UB | `err_e6_driver_out_of_range_enum_values` | [x] |
| E7 | `printLine` | oversized input: a string far larger than any internal buffer (1 MiB and 4 MiB + 1 of non-NUL bytes) — the C has no length limit and no bounds check at all | full contents echoed + `"\n"`; no truncation, no crash | `err_e7_print_line_oversized` | [x] |
| E8 | `printLine` | non-ASCII / non-UTF-8 arbitrary bytes `0x01..=0xFF` (invalid UTF-8 sequences, lone continuation bytes). C is byte-oriented; a Rust translation that assumed `str`/UTF-8 would corrupt or panic | bytes echoed verbatim + `"\n"` | `err_e8_print_line_invalid_utf8` | [x] |
| E9 | `printLine` | misaligned / unusual but valid pointer: pointer to a NUL byte located at the very end of a page-sized allocation (nothing readable past it) — verifies no over-read | writes `"\n"` only, no fault | `err_e9_print_line_pointer_at_boundary` | [x] |

Notes on cases that are deliberately *not* rows:

* There is no length/size parameter anywhere in the API, so "zero length" and
  "oversized length" can only be expressed as string contents (rows E2 and E7).
* `bad`, `good` take no parameters, so they have no invalid-input rows of their
  own beyond E4.
* A non-null but genuinely *invalid* (freed / unmapped) `line` pointer is not a
  row: the C would segfault, which is not a defined, comparable result.
