# ERRORS.md — Error-surface table (Phase C)

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Grep results for rejection constructs

```
$ grep -n 'assert\|return -1\|return NULL\|RETURN_ERROR\|errno\|if (\|#define' \
      c_src/src/driver.c c_src/include/driver.h
(no matches other than the include guard)
```

The C library performs **no validation whatsoever**: there are no `assert`s,
no error enums, no sentinel returns, no null checks, no range checks, and no
min/max constants. `foo` returns a plain count; `driver` returns `void`.

Consequently the "error surface" of this library consists entirely of
*inputs that the C code does not defend against*. Those are still real inputs
that cross the FFI boundary, so each one is listed below with the observable C
behaviour, and each one has a differential test that asserts the Rust `.so`
behaves **identically** (same count, or same fatal signal / same exit status,
observed in a forked child process so the harness survives).

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| E1 | `foo` | `in == NULL` (null pointer, no null check in C) | `strchr(NULL, c)` dereferences null → fatal `SIGSEGV`; process dies before returning | `err_e1_foo_null_pointer` (forked, compares wait-status of C child vs Rust child) | [x] |
| E2 | `driver` | `in == NULL` | first `foo(in,'A')` faults → fatal `SIGSEGV`, nothing printed | `err_e2_driver_null_pointer` (forked, compares wait-status) | [x] |
| E3 | `foo` | `c == 0` (the NUL terminator itself is a "valid" needle for `strchr`) | `strchr` matches the terminator, `res++`, then `s++` walks *past* the end of the string and keeps scanning; `strchr` can never return NULL for `c==0`, so the loop runs off the end of the object until it faults → non-returning / `SIGSEGV` | `err_e3_foo_nul_needle` (forked + `alarm(5)`, compares wait-status) | [x] |
| E4 | `foo` | `in` points at a non-NUL-terminated buffer (missing terminator) | reads past the end of the object; count depends on adjacent memory — no rejection | `err_e4_unterminated_buffer` (forked; both libs given the *same* buffer inside one child each, statuses compared) | [x] |
| E5 | `foo` | needle `c` with the high bit set, e.g. `0x80..0xFF` passed as a (signed) `char`, i.e. a *negative* `int` argument to `strchr` | **no error**: `char` is promoted to `int` (negative), glibc `strchr` compares `(char)c`, so byte `0x80..0xFF` is matched normally. Must NOT be treated as "not found". | `err_e5_negative_needle` (in-process, compares counts) | [x] |
| E6 | `foo` | needle `c` = `0x7F`, `0x01` (control bytes, one step outside printable ASCII range) | no error, plain count; verifies no hidden range check | `err_e6_control_byte_needles` | [x] |
| E7 | `foo` | empty string `""` (zero length input) | `strchr("", c)` returns NULL for any `c != 0` → returns `0` | `err_e7_empty_string` | [x] |
| E8 | `foo` | "oversized" input: string longer than any internal buffer would allow (256 KiB, all matches) | no error, no truncation, no overflow of the `int` counter at this size → exact count returned | `err_e8_oversized_input` | [x] |
| E9 | `driver` | input containing embedded high-bit / non-UTF-8 bytes (`0x80..0xFF`) — would break a naive `CStr::to_str()` based translation | no error: `printf("%d")` output is unaffected, counts of `'A'`/`'x'` still exact | `err_e9_driver_non_utf8` | [x] |
| E10 | `driver` | input whose bytes include `%` / `%s` / `%n` format specifiers | no error and **no format-string interpretation**: the input is never used as a format string, only `"A: %d\n"` / `"x: %d\n"` are | `err_e10_driver_format_specifiers` | [x] |
| E11 | `foo` | "out-of-range enum value" analogue: `c` is the full `i8` domain `-128..=127`, i.e. every representable value including ones with no printable meaning | no error for any value except `c == 0` (row E3); every other value is a plain count | `err_e11_full_needle_domain` (all 256 values × many random strings) | [x] |

## Observed outcomes for the undefined-behaviour rows

Recorded by running `cargo test --test error_paths -- --nocapture`:

| row | C `.so` child | Rust `.so` child | verdict |
|---|---|---|---|
| E1 `foo(NULL, c)` | `Signaled(11)` = SIGSEGV | `Signaled(11)` | identical |
| E2 `driver(NULL)` | `Signaled(11)` = SIGSEGV | `Signaled(11)` | identical |
| E3 `foo(s, 0)` | `Signaled(11)` = SIGSEGV (runs off the object, never returns) | `Signaled(11)` | identical |
| E4 unterminated 4 KiB buffer | `Exited(74)` — i.e. it *returned* a count of 586 (`586 & 0x7f == 74`), the scan stopping at the first NUL after the object | `Exited(74)` | identical (same count, and neither side added a bounds check) |

Notes:

* Rows E1–E4 are C-level undefined behaviour. The requirement "same
  error/rejection" is interpreted as *same observable outcome*: the tests fork
  a child per implementation and assert the two children terminate with the
  **same** `wait(2)` status (same signal / same exit code), so a Rust version
  that returned `0` instead of faulting, or panicked with a Rust message, would
  fail the test.
* There are no enums in this API, so E11 covers the equivalent "value with no
  valid variant crosses FFI" class for the only non-pointer parameter.
