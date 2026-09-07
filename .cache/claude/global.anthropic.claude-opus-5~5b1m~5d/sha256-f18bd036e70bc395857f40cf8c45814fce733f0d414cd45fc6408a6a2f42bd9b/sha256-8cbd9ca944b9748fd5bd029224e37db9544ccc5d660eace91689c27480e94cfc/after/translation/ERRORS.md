# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping the whole C source for every rejection /
error path construct:

```
grep -n 'return\|NULL\|assert\|if *(\|exit\|abort\|errno\|-1' c_src/src/driver.c c_src/include/driver.h
```

Findings — the complete set of conditional / rejection constructs in the C:

| location | construct |
|----------|-----------|
| `driver.c:31` `printLine` | `if (line != NULL) { printf("%s\n", line); }` — the ONLY null check in the library |
| `driver.c:58` `good`      | `data = NULL;` then immediately reassigned — dead store, not a check |
| `driver.c:71` `driver`    | `if (useGood) { good(); } else { bad(); }` — branch, not an error |
| everywhere               | no `assert`, no `return -1`, no `return NULL`, no error enum, no `errno` use, no range check, no min/max constant, no allocation-failure check |

All five public functions return `void`; the library has **no error codes and
no sentinel return values**. The only *observable* "rejection" of input is
`printLine` silently producing no output for a `NULL` argument. Everything else
is exercised as "the same (possibly undefined-behaviour-adjacent) observable
output for out-of-domain inputs".

## Error / rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `printLine` | `line == NULL` | no output at all, returns normally (the `if (line != NULL)` guard rejects it) | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at an empty string `""` (zero-length, not NULL — passes the guard) | prints a single `"\n"` | `err_e2_print_line_empty` | [x] |
| E3 | `printLine` | `line` contains `printf` conversion specifiers (`"%s %d %n"`) — passed as the *argument*, not the format | prints the literal bytes + `"\n"` (no format interpretation) | `err_e3_print_line_percent` | [x] |
| E4 | `printLine` | oversized `line` (64 KiB, larger than any stdio buffer) | prints all 65536 bytes + `"\n"` | `err_e4_print_line_oversized` | [x] |
| E5 | `printLine` | `line` containing embedded non-ASCII / high bytes (0x80..0xFF) up to the NUL | prints the raw bytes verbatim + `"\n"` | `err_e5_print_line_high_bytes` | [x] |
| E6 | `printIntLine` | `INT_MIN` (`-2147483648`) — one step past the negative end of the `int` range | prints `-2147483648\n` | `err_e6_print_int_line_int_min` | [x] |
| E7 | `printIntLine` | `INT_MAX` (`2147483647`) — the positive extreme | prints `2147483647\n` | `err_e7_print_int_line_int_max` | [x] |
| E8 | `printIntLine` | `0` and `-1` (sentinel-looking values) | prints `0\n` / `-1\n` | `err_e8_print_int_line_sentinels` | [x] |
| E9 | `driver` | `useGood == 0` (the false branch — "rejects" the good path) | runs `bad()`, prints `0\n` | `err_e9_driver_zero` | [x] |
| E10 | `driver` | `useGood` = out-of-range "enum-like" ints with no meaningful variant: `2`, `-1`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFFFF` (C `int` accepts any value; every non-zero one is truthy) | runs `good()` for every non-zero value, prints `0\n` | `err_e10_driver_out_of_range_enum` | [x] |
| E11 | `bad` | called directly with its intentionally under-sized `alloca(10)` (CWE-806 over-read/overflow of 40 bytes into a 10-byte allocation) | still prints `0\n` (the surplus stores land in caller-frame stack slack); must not crash | `err_e11_bad_direct_overflow` | [x] |
| E12 | `good` | called directly (the `data = NULL` dead store must not be observable as a null deref) | prints `0\n` | `err_e12_good_direct` | [x] |

All 12 rows have passing differential tests in `tests/differential.rs`.
