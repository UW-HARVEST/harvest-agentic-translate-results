# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.
The complete grep for rejection constructs over the whole C source is:

```
$ grep -n "return\|assert\|NULL\|exit\|abort\|errno\|if\s*(" c_src/src/driver.c c_src/include/driver.h
c_src/src/driver.c:32:    if(line != NULL)
c_src/src/driver.c:61:    data = NULL;
c_src/src/driver.c:75:    if (useGood)
```

That is the entire set. This library has:

- **no** `RETURN_ERROR`-style macro, **no** error enum, **no** error codes;
- **no** `return <value>` at all — every one of the five functions is `void`,
  so there is no return value that could carry a status;
- **no** `assert`, no `exit`, no `abort`, no `errno` use;
- **no** explicit range check and no min/max constant (the only numeric
  literals in the file are the array/loop bound `10`, the `0` initialiser, and
  the copyright year `2025`);
- exactly **one** guard that rejects an input: the null check at line 32.

Line 61 (`data = NULL;`) is a local initialisation inside `good()`, not a
rejection. Line 75 (`if (useGood)`) is a mode selector, not a rejection — it is
covered as a configuration axis in `CONFIGS.md`.

Consequently the table below is short **because the C is short**, not because
rows were pruned. Rows 2–7 are the generic FFI-boundary boundaries the task
requires regardless of the table; each states the behaviour the C actually
exhibits, and each is asserted identical for C and Rust.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `printLine` | `line == NULL` — the guard at `driver.c:32` is false | silent no-op: **no** `printf`/`puts` call, **zero** bytes written to `stdout`, no crash, returns normally | `err_01_print_line_null_is_silent_noop` | [x] |
| 2 | `printLine` | `line` points at a 0-length string (`""`) — passes the null guard with an empty payload | writes exactly one byte, `"\n"` | `err_02_print_line_empty_string` | [x] |
| 3 | `printLine` | `line` payload contains `printf` conversion specifiers (`"%s %d %n %%"`) — the value is a *data* argument to a `"%s\n"` format, so it must **not** be interpreted as a format string | the specifier text is echoed verbatim followed by `"\n"`; no format-string evaluation | `err_03_print_line_format_specifiers_not_interpreted` | [x] |
| 4 | `printLine` | `line` payload is oversized (64 KiB, i.e. far past any stdio buffer) and contains non-ASCII / high bytes `0x80..0xFF` | all payload bytes echoed verbatim, then `"\n"`; no truncation at the 4 KiB/8 KiB `stdout` buffer boundary | `err_04_print_line_oversized_and_high_bytes` | [x] |
| 5 | `printIntLine` | out-of-usual-range `int` values one step past the extremes: `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `-1`, `0` | `%d` rendering of the two's-complement value (`INT_MIN` → `-2147483648`), then `"\n"` — no clamping, no overflow trap | `err_05_print_int_line_extremes` | [x] |
| 6 | `driver` | out-of-range "enum-like" values for the `useGood` flag: the C parameter is `int`, so every one of the 2^32 values is a real input, including ones no sane caller passes (`2`, `-1`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFFFF0000`) | `if (useGood)` is C truthiness, **not** an equality test against `1`: every non-zero value selects `good()`, only `0` selects `bad()`. No value is rejected, nothing is validated, no error is reported | `err_06_driver_out_of_range_flag_values`, `err_09_driver_low_byte_zero_is_still_truthy` | [x] |
| 7 | `driver` | the `int` argument is passed with a dirty upper half of the 64-bit register (`0xFFFFFFFF_00000000`-style values whose low 32 bits are `0`) — i.e. a value that is truthy as 64-bit but zero as `int` | only the low 32 bits are significant; such a value is `0` as an `int` and therefore selects `bad()` | `err_09_driver_low_byte_zero_is_still_truthy` | [x] |

`bad()` and `good()` take no arguments and contain no guard, so they contribute
no rejection rows; their (single, defect-preserving) behaviour is a
`CONFIGS.md` row instead.

## Status

All 7 rows have a passing error-path differential test that constructs the
condition, calls **both** the C `.so` and the Rust `.so` through their exported
symbols, and asserts the *same* observable result — identical captured `stdout`
bytes, and for row 1 the specific sentinel "zero bytes written" rather than
merely "both did something".
