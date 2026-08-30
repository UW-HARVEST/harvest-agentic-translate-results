# ERRORS.md — Error / rejection surface table

Mechanically derived from `c_src/src/driver.c`. Grep results for every rejection
construct in the whole C tree:

```
$ grep -n 'return\|assert\|NULL\|ERROR\|errno\|exit\|abort\|if\s*(' c_src/src/driver.c c_src/include/driver.h
c_src/src/driver.c:31:    if(line != NULL)
```

That is the **only** conditional and the **only** rejection in the library.
There are:

* **no** `return` statements (every function is `void` and falls off the end),
* **no** `assert` / `NULL`-returning functions / error enums / error codes,
* **no** explicit range checks, and
* **no** min/max constants.

So the whole error surface is one row, plus the generic FFI boundary cases the
task requires us to cover anyway. "Expected C result" for a `void` function is
expressed as *the bytes written to stdout* (the sole observable), plus
"returns normally / does not crash".

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `printLine` | `line == NULL` (`if(line != NULL)` guard fails) | silently returns; **zero bytes** written to stdout; no crash |

## Generic FFI-boundary cases (not in the C table, covered regardless)

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| G1 | `printLine` | `NULL` pointer (same as E1, asserted byte-exactly against Rust) | no output | [x] |
| G2 | `printLine` | zero-length string: pointer to a lone `'\0'` | exactly one byte, `"\n"` | [x] |
| G3 | `printLine` | oversized input: 1 MiB NUL-terminated string | the 1 MiB payload + `"\n"` | [x] |
| G4 | `printLine` | payload containing `%s`, `%n`, `%d`, `%%` (format-specifier-looking data; `line` is an *argument*, never a format string) | printed literally + `"\n"` | [x] |
| G5 | `printLine` | payload with all non-NUL byte values `0x01..=0xFF` incl. high-bit / invalid UTF-8 | raw bytes verbatim + `"\n"` | [x] |
| G6 | `printLine` | payload containing embedded `'\n'`, `'\r'`, `'\t'` | raw bytes verbatim + `"\n"` | [x] |
| G7 | `printIntLine` | `INT_MIN` (`-2147483648`) — one step past the negative range of `int` | `"-2147483648\n"` | [x] |
| G8 | `printIntLine` | `INT_MAX` (`2147483647`) | `"2147483647\n"` | [x] |
| G9 | `printIntLine` | `0`, `-1`, `1`, `±1` around all power-of-two digit boundaries | decimal, no padding, `'\n'` terminated | [x] |
| G10 | `printIntLine` | 64-bit value whose low 32 bits are the payload, pushed through the `int` ABI slot (caller passes a wider value than the C `int` parameter — value one step past what the declared type holds) | both truncate to the same low 32 bits | [x] |
| G11 | `bad` / `good` / `driver` | called with a non-`void` prototype (extra register arguments supplied over the FFI boundary — C `()` accepts any arg list) | arguments ignored, identical output | [x] |
| G12 | "out-of-range enum value" class | **N/A by construction** — the C API declares **no `enum`, no flags and no mode parameter**; the only non-`void` parameters are `const char *` (covered by G1–G6) and `int` (whose *entire* 2^32 value range is valid and is covered by G7–G10 plus the randomised sweep in Phase B). Documented here so the class is explicitly discharged, not silently skipped. | — | [x] |
| G13 | all 5 symbols | repeated / interleaved invocation (stdio buffering state carried across calls) | identical byte stream | [x] |
| G14 | all 5 symbols | the output stream itself fails — `stdout` points at a read-only `FILE*`, so every `printf`/`puts` returns `< 0`. Neither implementation inspects that return value. | nothing written, `ferror(stdout)` set, all functions return normally, and the library keeps working on a healthy stream afterwards | [x] |

## Status

* Row `E1`: **[x]** covered by `tests/phase_c_errors.rs::e1_print_line_null`.
* Rows `G1`–`G14`: **[x]** covered by `tests/phase_c_errors.rs`
  (`g1_…` … `g14_…`, plus `extra_print_line_unaligned_and_interior_pointer`).

All rows checked → Phase D may proceed.

## How "same rejection" is asserted

Since the API is entirely `void`, the differential assertion for every row is
made on the *complete observable state*, not on "both failed somehow":

1. the exact byte sequence written to `stdout` (compared byte-for-byte between
   the two `.so`s, and against an independent reference model),
2. the `ferror(stdout)` flag afterwards (row G14),
3. normal return / no abort (a crash in either `.so` fails the test process).

The harness swaps glibc's `stdout` `FILE*` rather than `dup2`-ing fd 1, so
libtest's own progress output can never contaminate a capture; the negative
controls in `tests/phase_a_selfcheck.rs` prove the capture is neither empty nor
blind to a real divergence.
