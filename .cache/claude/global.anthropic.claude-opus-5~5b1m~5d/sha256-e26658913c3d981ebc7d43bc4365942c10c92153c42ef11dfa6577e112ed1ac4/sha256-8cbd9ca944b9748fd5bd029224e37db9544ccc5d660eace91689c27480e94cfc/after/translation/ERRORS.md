# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c`. Grep audit of every rejection
construct the C source could contain:

```
grep -nE 'RETURN_ERROR|return +-?[0-9]|return +NULL|assert|errno|exit\(|abort\(' c_src/src/driver.c   -> no matches
grep -nE 'if *\(|switch|#if'                                                     c_src/src/driver.c   -> 1 match (line 31)
grep -nE 'NULL'                                                                  c_src/src/driver.c   -> 1 match (line 31)
grep -nE 'enum|#define'                                                          c_src/src/driver.c   -> no matches
```

The library has **no** return codes, **no** error enums, **no** `assert`, **no**
`errno` use, **no** numeric range checks and **no** min/max constants. All four
exported functions return `void`. The entire error surface is therefore the
single NULL guard in `printLine`, plus the generic FFI boundaries required by
the task.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` (line 31: `if (line != NULL)`) — the guard's false branch | returns normally, emits **zero bytes** to stdout, no crash | `err_1_print_line_null` | [x] |
| 2 | `printLine` | `line` points at `""` (a lone `'\0'`) — zero-length, *valid* but the degenerate boundary of the true branch | returns normally, emits exactly one byte `"\n"` | `err_2_print_line_empty_string` | [x] |
| 3 | `printLine` | `line` points at a buffer whose first byte is `'\0'` but which has trailing garbage after it (embedded-NUL / "oversized length") | `puts` stops at the first `'\0'`: emits only `"\n"`, ignores the tail | `err_3_print_line_embedded_nul` | [x] |
| 4 | `printLine` | `line` is a non-NULL but *misaligned / arbitrary low* pointer value that is still a valid readable string (`(char*)p + 1` into a buffer) — proves the guard tests only against NULL, not against "looks bogus" | prints from the offset byte | `err_4_print_line_unaligned_offset` | [x] |
| 5 | `printLine` | `line` is a 4095-byte string with no interior NUL (oversized length, no internal C buffer to overflow) | prints all 4095 bytes + `"\n"` | `err_5_print_line_oversized` | [x] |
| 6 | `bad`, `good`, `driver` | called with a *wrong-arity* FFI signature — declared `void(void)` in the header, so any extra argument is ignored by the SysV ABI (the "value one step past a documented range" analogue for a nullary API) | ignores the extra argument, identical output | `err_6_nullary_extra_args_ignored` | [x] |

## Not applicable (documented for completeness)

- **Out-of-range enum values across FFI:** the C source declares no `enum` and
  no function takes an integer parameter, so there is no enum-shaped input to
  push out of range. Row 6 covers the closest real analogue (an extra argument
  passed to a `void(void)` symbol).
- **Non-zero return / sentinel comparison:** all four symbols return `void`.
  Rows therefore assert on *observable stdout bytes*, which is the only result
  channel this library has. `puts`'s own return value is discarded by the C, so
  it is not part of the observable surface.
- **Truly invalid pointers** (e.g. `(char*)1`, freed memory) are undefined
  behaviour in the C and are *not* tested: the C is not required to reject them,
  so a differential test would compare two undefined behaviours.
