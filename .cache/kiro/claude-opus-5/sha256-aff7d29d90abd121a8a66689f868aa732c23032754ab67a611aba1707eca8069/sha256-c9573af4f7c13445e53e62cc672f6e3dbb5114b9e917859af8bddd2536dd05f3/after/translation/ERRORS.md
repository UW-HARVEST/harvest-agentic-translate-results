# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source. The greps used over
`c_src/src/driver.c` and `c_src/include/driver.h`:

| grep pattern | hits |
|---|---|
| `return` | **none** (all five functions are `void` and fall off the end) |
| `assert` | none |
| `RETURN_ERROR`, `errno`, `exit(`, `abort` | none |
| `-1` / error sentinels | none |
| `NULL` | 1 hit — `driver.c:31 if(line != NULL)` |
| `if` / `else` / `switch` / `case` / `while` / `for` / `#if` / `#ifdef` | 1 hit — the same `driver.c:31` |
| `enum` / `struct` / `typedef` / `static` / `const` | only `const char *` in `printLine`'s parameter |

So the library's **entire** rejection surface is a single guard, and no function
can report failure to its caller (every function returns `void`; there are no
out-parameters and no global error state).

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `printLine` | `line == NULL` (`driver.c:31` false branch) | Silently prints nothing: 0 bytes written to `stdout`, no crash, returns normally | `err_e1_print_line_null` | [x] |

## Generic C-API boundaries covered anyway

These are not rows of the table above (the C code contains no check for them),
but they are exercised to confirm C and Rust agree, since "no check" is itself a
behaviour that must match.

| # | condition | expected C result | test | status |
|---|-----------|-------------------|------|--------|
| G1 | `printLine(NULL)` repeated, and interleaved with valid calls (no state corruption) | nothing printed for the NULL calls; valid calls unaffected | `err_g1_null_interleaved` | [x] |
| G2 | `printLine("")` — zero-length (but non-NULL) string | a single `\n` | `err_g2_empty_string` | [x] |
| G3 | `printLine` with an oversized string (4095/4096/4097/65535/65536/1 MiB bytes, crossing libc stream-buffer boundaries) | the whole string then `\n`, unbounded — the C imposes no length limit | `err_g3_oversized_string` | [x] |
| G4 | `printLine` with `printf` format specifiers in the *content* (`%s`, `%d`, `%n`, `%%`, `%99999999d`) | printed literally — `line` is an argument, never the format string. A translation that passed `line` as the format string would diverge or crash here | `err_g4_format_specifiers_in_content` | [x] |
| G5 | `printLine` with arbitrary non-UTF-8 / high bytes (`0x01`–`0xFF`, no interior NUL) | raw bytes copied through verbatim; not valid UTF-8, so a translation going through Rust `str` would diverge | `err_g5_non_utf8_bytes` | [x] |
| G6 | `printIntLine` at the extremes of the `int` range: `INT_MIN` (`-2147483648`), `INT_MAX` (`2147483647`), `0`, `-1`, `1` | the decimal rendering; `INT_MIN` must not overflow/panic on negation | `err_g6_int_extremes` | [x] |
| G7 | `printIntLine` one step past each decimal-width boundary (`±9`/`±10`, `±99`/`±100`, … `±999999999`/`±1000000000`) — digit-count edges of `%d` | exact decimal rendering with no padding | `err_g7_int_width_boundaries` | [x] |
| G8 | Out-of-range enum values across the FFI boundary | **N/A by construction** — the public API declares no `enum` parameter (no `enum` anywhere in the C source). The nearest equivalent is an arbitrary `int` reaching `printIntLine`, where *every* one of the 2^32 bit patterns is a valid input; covered exhaustively-by-sampling in `cfg_12_int_random` and by G6/G7 at the edges | `err_g8_int_arbitrary_bit_patterns` | [x] |

## Deliberately not tested (undefined behaviour in C)

Passing a non-NULL but invalid pointer (dangling, unmapped, or not
NUL-terminated) to `printLine` is undefined behaviour in the C original — the
`if(line != NULL)` guard does not and cannot detect it. There is no defined C
result to compare against, so no differential test can be written for it.
