# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/driver.c` (83 lines, the only C source) and
`c_src/include/driver.h`. Grep results for every rejection mechanism:

```
$ grep -nE 'return|assert|NULL|ERROR|errno|exit|abort|<|>|==|!=' c_src/src/driver.c
32:    if(line != NULL)          <- the ONLY input rejection in the library
50:        for (i = 0; i < 10; i++)   <- fixed loop bound, not an input check
61:    data = NULL;              <- assignment, immediately overwritten
66:        for (i = 0; i < 10; i++)   <- fixed loop bound, not an input check
75:    if (useGood)              <- mode dispatch, not a rejection
```

Facts about this library's error surface:

* Every public function returns `void`. There is **no** error code, no sentinel
  return, no `errno` use, no output parameter, and no error enum anywhere.
* There is exactly **one** guard on an input value: the `line != NULL` test in
  `printLine`.
* There are **no** `assert`s, no `RETURN_ERROR`-style macros, no explicit range
  checks, and no min/max constants.
* There are **no enum types** in the public API, so "out-of-range enum value"
  degenerates to "out-of-range `int`" for `driver(int useGood)`, which C treats
  as a plain truthiness test — every `int` bit pattern is a *valid* input and is
  covered here as well as in `CONFIGS.md`.

Therefore "the same error/rejection" is observable only as **the exact bytes
written to `stdout`** (the empty byte string being the rejection outcome).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `printLine` | `line == NULL` (`if(line != NULL)` false at driver.c:32) | returns normally, writes **0 bytes** to stdout, no crash | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at an empty string `""` (guard passes, zero-length payload) | writes exactly `"\n"` (1 byte) | `err_e2_print_line_empty` | [x] |
| E3 | `printLine` | `line` payload contains `printf` conversion specifiers (`%s`, `%n`, `%d`, `%%`) — data must NOT be interpreted as a format string | specifiers echoed literally, then `'\n'` | `err_e3_print_line_format_specifiers` | [x] |
| E4 | `printLine` | `line` payload contains embedded `\n`, `\t`, `\r`, `\0`-adjacent and non-UTF-8 (0x80..0xFF) bytes | bytes copied verbatim up to the first NUL, then `'\n'` | `err_e4_print_line_non_utf8_and_control` | [x] |
| E5 | `printLine` | oversized payload: length far past any stdio buffer (`BUFSIZ`, 4 KiB, 64 KiB, 1 MiB) | full payload then `'\n'`, no truncation | `err_e5_print_line_oversized` | [x] |
| E6 | `printIntLine` | boundary / one-past-range integers: `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `-1`, `0` | `%d` rendering incl. the non-negatable `-2147483648` | `err_e6_print_int_line_boundaries` | [x] |
| E7 | `driver` | `useGood == 0` — the *only* value routed to the intentionally buggy `bad()` (CWE-806 `alloca(10)` under-allocation) | runs `bad()`, prints `"0\n"`, must not abort/trap | `err_e7_driver_zero_selects_bad` | [x] |
| E8 | `driver` | out-of-`bool`-range `int` values `-1`, `2`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFFFF_FF00` — a C `int` accepts any bit pattern where an enum/bool was implied | any non-zero → `good()`; identical `"0\n"`; Rust must use `!= 0`, not `== 1` | `err_e8_driver_out_of_range_int` | [x] |
| E9 | `bad` | called directly (bypassing `driver`), the out-of-bounds-write path itself | prints `"0\n"` and returns normally | `err_e9_bad_direct_no_trap` | [x] |
| E10 | `good`/`bad` | called repeatedly / alternately, so a corrupted frame from `bad()` would surface on a later call | every call prints `"0\n"`, unchanged | `err_e10_repeated_alternating_calls` | [x] |

Non-cases (documented so they are not silently skipped): passing a non-NUL-terminated
buffer to `printLine`, or a wild non-NULL pointer, is undefined behaviour in the C
(`printf`/`puts` read until a NUL). Those are not tested because the C has no defined
result to match against.
