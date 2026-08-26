# ERRORS.md — Error / rejection surface of the C library

Derived mechanically from the C source, not from docs:

```
$ grep -rnE "return|NULL|assert|errno|-1|exit|abort|#if|enum|MAX|MIN|if *\(|switch" c_src/src c_src/include
c_src/src/driver.c:31:    if(line != NULL)
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

The library contains **exactly one rejection branch** in the entire codebase:
the `line != NULL` guard in `printLine`. There are:

* no `return` statements at all (all five functions are `void` and fall off the end),
* no error codes / sentinels / error enums (nothing returns a value),
* no `assert`s, no `errno` use, no `exit`/`abort`,
* no range checks, no min/max constants,
* no enums anywhere, hence no out-of-range-enum FFI surface to probe
  (`printIntLine`'s `int` parameter is a plain `int`, and *every* `int` bit pattern
  is a valid input that the C accepts and formats with `%d` — there is no invalid
  value to reject; this is verified over the full range incl. `INT_MIN`/`INT_MAX`
  below and in `CONFIGS.md`),
* no `#ifdef` build-time configuration (the only preprocessor conditional is the
  header's include guard), hence a single build configuration.

"Rejection" for this library therefore means *"produces no output and does not
crash"*; the observable result of every call is the exact byte sequence written to
`stdout` (all functions return `void`). Every row below asserts the C and Rust
byte streams are identical, so "same error/rejection" == "same observable result",
not merely "both failed somehow".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` — the one and only explicit rejection in the library (`if(line != NULL)` is false) | falls through the `if`, writes **nothing** at all to `stdout` (0 bytes), returns normally | `err_01_print_line_null` | [x] |
| 2 | `printLine` | `line` non-NULL but points at a zero-length C string (`""`) — the boundary one step past NULL: passes the guard, zero payload | `printf("%s\n", "")` ⇒ exactly `"\n"` (1 byte) | `err_02_print_line_empty` | [x] |
| 3 | `printLine` | `line` points at a buffer whose **first byte is NUL** followed by more non-NUL bytes (embedded-NUL truncation) | `%s` stops at the NUL ⇒ exactly `"\n"`; trailing bytes never appear | `err_03_print_line_embedded_nul_first` | [x] |
| 4 | `printLine` | `line` points at a buffer with an **embedded NUL in the middle** (`"abc\0def"`) — the C string ends early | prints only `"abc\n"`; `"def"` is not printed | `err_04_print_line_embedded_nul_mid` | [x] |
| 5 | `printLine` | `line` contains `printf` **format specifiers** (`%s`, `%d`, `%n`, `%p`, `%%`, `%1000000d`) — a classic format-string hazard; C passes it as an *argument* to `%s`, never as a format | the specifiers are printed **literally** as data, no argument consumption, no crash | `err_05_print_line_format_specifiers` | [x] |
| 6 | `printLine` | oversized input: 1 MiB string (far past any stdio buffer size) | whole string + `"\n"` written, no truncation | `err_06_print_line_oversized` | [x] |
| 7 | `printLine` | non-ASCII / **invalid UTF-8** bytes (e.g. `0x80 0xFF 0xFE`, lone surrogates in UTF-8 form) — Rust `str` would reject these, C does not | bytes passed through verbatim + `"\n"` | `err_07_print_line_invalid_utf8` | [x] |
| 8 | `printLine` | every single non-NUL byte value `0x01..=0xFF` as a 1-byte string (exhaustive byte-domain sweep) | each byte echoed verbatim + `"\n"` | `err_08_print_line_all_byte_values` | [x] |
| 9 | `printIntLine` | `INT_MIN` (`-2147483648`) — the extreme negative boundary, the value whose negation overflows | `printf("%d", INT_MIN)` ⇒ `"-2147483648\n"` | `err_09_print_int_line_int_min` | [x] |
| 10 | `printIntLine` | `INT_MAX` (`2147483647`) — the extreme positive boundary | `"2147483647\n"` | `err_10_print_int_line_int_max` | [x] |
| 11 | `printIntLine` | one step past each boundary via wraparound of the 32-bit domain: `INT_MIN-1`≡`INT_MAX` and `INT_MAX+1`≡`INT_MIN` bit patterns, plus `0u32`/`0xFFFFFFFF` reinterpreted as `int` | value reinterpreted as `int` and formatted with `%d` (`0xFFFFFFFF` ⇒ `"-1\n"`, `0x80000000` ⇒ `"-2147483648\n"`) | `err_11_print_int_line_wraparound` | [x] |
| 12 | `printIntLine` | a 64-bit-wide argument passed where the C prototype says `int` (upper 32 bits dirty) — the FFI-boundary width mismatch an external caller can produce | only the low 32 bits are formatted (`%d` reads a 32-bit `int`); C and Rust must agree | `err_12_print_int_line_dirty_upper_bits` | [x] |
| 13 | `bad` / `good` / `driver` | no parameters ⇒ **no input can be invalid**; the only way to "misuse" them is repeated/interleaved invocation, which must not accumulate hidden state | identical byte stream on every invocation | `err_13_no_arg_fns_have_no_error_path` | [x] |

All 13 rows have a passing differential test (see `tests/differential.rs`).
