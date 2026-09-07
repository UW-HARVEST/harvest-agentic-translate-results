# ERRORS.md — Phase C error-surface table

Derived mechanically. Every error/rejection construct in the C source was located
with:

```sh
grep -n 'return\|assert\|NULL\|if\|switch\|error\|ERROR\|-1\|exit\|abort\|MAX\|MIN\|#if' \
    c_src/src/driver.c c_src/include/driver.h
```

The only non-comment match in the whole library is **one** conditional:

```c
c_src/src/driver.c:31:    if (line != NULL)
```

There are no error-return macros, no `assert`, no error enums, no status codes,
no range checks, no min/max constants, no `exit`/`abort`, and no integer
sentinels. Every public function returns `void`, so the only observable
"rejection" is the *suppression of output*. That yields exactly one derived row;
the remaining rows are the generic FFI boundaries Phase C mandates regardless of
the table.

## Derived rows (from what the C actually checks)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` (the `if (line != NULL)` guard at driver.c:31 fails) | returns normally; **zero bytes** written to stdout | `err_row1_print_line_null` | [x] |

## Mandated generic-boundary rows

`printLine` takes a single `const char *` and no length; `bad`, `good`, `driver`
take no arguments at all. There is therefore no length parameter to zero or
oversize, and no enum to push out of range. The reachable boundaries are:

| # | function | trigger | expected C result | test | [x] |
|---|----------|---------|-------------------|------|-----|
| 2 | `printLine` | `line` = `(char*)0` re-passed many times in a row (repeated rejection is not sticky) | zero bytes each time, no state change | `err_row2_print_line_null_repeated` | [x] |
| 3 | `printLine` | `line` = pointer to a lone `'\0'` (zero-length string — the boundary one step inside the valid range) | writes exactly `"\n"` (1 byte) | `err_row3_print_line_empty` | [x] |
| 4 | `printLine` | `line` interleaved: `NULL`, then valid, then `NULL` — a rejection between two accepted calls must not swallow or reorder the accepted output | writes only the valid string + `"\n"` | `err_row4_null_interleaved_with_valid` | [x] |
| 5 | `printLine` | `line` containing `printf` conversion specifiers (`%s %d %n %%`) — data must be treated as data, never as a format string | the specifiers are written **literally** + `"\n"` | `err_row5_format_specifiers_not_interpreted` | [x] |
| 6 | `printLine` | `line` containing an embedded `'\0'` before the end of the buffer | output truncates at the `'\0'`, then `"\n"` | `err_row6_embedded_nul_truncates` | [x] |
| 7 | `printLine` | `line` = every single non-NUL byte value `0x01..=0xFF`, including bytes that are invalid UTF-8 | each byte written verbatim + `"\n"`; no UTF-8 validation, no replacement char, no panic | `err_row7_all_single_byte_values` | [x] |
| 8 | `printLine` | oversized input: a 1 MiB string (far past stdio's buffer, forcing internal flushes) | full 1 MiB written verbatim + `"\n"` | `err_row8_oversized_input` | [x] |
| 9 | `helperBad` / `helperGood` | symbol looked up across the FFI boundary although the C declares it `static` | `dlsym` fails on **both** libraries | `err_row9_static_helpers_not_resolvable` | [x] |
| 10 | `bad`, `good`, `driver` | called with no arguments but through a mismatched-arity C ABI call (extra register garbage), i.e. the void-parameter boundary | ignored; identical output to the plain call | `err_row10_void_functions_ignore_extra_args` | [x] |
| 11 | `printLine` | only the exact value `0` is the rejection sentinel: a non-NULL *interior* pointer, high and unaligned, deep inside a 1 MiB allocation | accepted; the bytes from that offset to the `'\0'` are printed + `"\n"` | `err_extra_only_exact_null_is_the_sentinel` | [x] |

Notes on inputs deliberately *excluded* as C-level undefined behaviour rather
than "inputs the C handles": a non-NULL pointer to unmapped memory, and a
non-NULL pointer to a byte sequence with no terminating `'\0'`. The C would
fault or read out of bounds; there is no defined result to match.
