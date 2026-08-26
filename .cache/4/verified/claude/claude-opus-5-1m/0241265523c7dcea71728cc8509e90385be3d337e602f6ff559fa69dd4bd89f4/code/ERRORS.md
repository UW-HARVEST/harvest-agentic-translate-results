# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/main.c` (71 lines, the only translation
unit). The complete grep of every branch, return and library call that can
reject or discard input:

```text
28:    if (line != NULL)          <-- rejection #1 (null pointer -> no output)
30:        printf("%s\n", line);
37:    return charString;         <-- returns address of a local (UB); gcc
                                      -Wreturn-local-addr substitutes NULL
48:    return charString;         <-- static storage, always valid
59:    scanf("%d", &x);           <-- rejections #3..#8 (input/matching failure)
61:    if (x)
70:    return 0;                  <-- the only return value main can produce
```

There are **no** `assert`s, no `RETURN_ERROR`-style macros, no error enums, no
`return -1` / `return NULL` statements, and no explicit range or min/max
constants in the C source. The program's entire rejection behavior consists of
(a) `printLine`'s null check and (b) the failure modes of a single `%d`
`scanf` directive, whose observable effect is that `x` keeps its initializer
`0` and therefore the `bad()` branch is taken.

`scanf`'s return value is **discarded** by the C code, so a `scanf` failure is
observable *only* through `x` remaining `0`. That means every rejection row
below has the same expected observable result: **no output at all, exit code 0**
(because `bad()` -> `printLine(helperBad())` -> `printLine(NULL)` -> nothing).

Legend for "expected C result": `""` means an empty byte stream on stdout.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` — the `if (line != NULL)` guard at line 28 fails | returns without writing anything; stdout `""` | `err_01_print_line_null` | [x] |
| 2 | `bad` (via `helperBad`) | unconditional: `helperBad()` returns the address of the local `charString`, which gcc lowers to a literal `NULL` return, so `bad()` always hits rejection #1 | stdout `""` | `err_02_bad_prints_nothing` | [x] |
| 3 | `main` / `scanf("%d")` | **input failure**: stdin is empty (immediate EOF before the whitespace skip finds any character) | `x` untouched (`0`) -> `bad()` -> stdout `""`, exit 0 | `err_03_scanf_eof_empty` | [x] |
| 4 | `main` / `scanf("%d")` | **input failure**: stdin contains only whitespace (`" "`, `"\n"`, `"\t\v\f\r "`, …) so EOF is hit inside the whitespace skip | stdout `""`, exit 0 | `err_04_scanf_eof_whitespace_only` | [x] |
| 5 | `main` / `scanf("%d")` | **matching failure**: first non-whitespace character is not a sign and not a digit (`"abc"`, `"x"`, `"."`, `"/"`, `":"`, `"\x80"`, `"\x00"`, `"-"`-less garbage) — the digit-collection loop stores nothing, so the "there was no number" check fires | stdout `""`, exit 0 | `err_05_scanf_matching_failure_non_digit` | [x] |
| 6 | `main` / `scanf("%d")` | **matching failure**: a lone sign, i.e. the workspace holds exactly one character and it is `'+'` or `'-'` (`"+"`, `"-"`, `"+ "`, `"-x"`, `"--5"`, `"+-3"`, `"-"`+EOF) | stdout `""`, exit 0 | `err_06_scanf_matching_failure_lone_sign` | [x] |
| 7 | `main` / `scanf("%d")` | **out-of-`int` range, truncating to 0**: the collected digits parse to a `long` whose low 32 bits are all zero, so `(int) num.l == 0` and the `if (x)` test fails even though the conversion *succeeded* (`"4294967296"`, `"-4294967296"`, `"8589934592"`, `"-8589934592"`, `"1099511627776"`, `"281474976710656"`, `"-9223372036854775808"`, and any multiple of 2^32) | stdout `""`, exit 0 | `err_07_scanf_int_truncates_to_zero` | [x] |
| 8 | `main` / `scanf("%d")` | **`ERANGE` saturation**: magnitude exceeds `LONG_MAX`/`LONG_MIN`, so `strtol` clamps; positive clamp gives `LONG_MAX` -> `(int) -1` (**non-zero**, `good()` runs) and negative clamp gives `LONG_MIN` -> `(int) 0` (`bad()` runs). Both directions must be reproduced. Note that `"18446744073709551616"` (2^64) saturates *up* to `LONG_MAX` and therefore prints, whereas `"-18446744073709551616"` saturates *down* to `LONG_MIN` and does not. | `"99999999999999999999"`, `"18446744073709551616"` -> `"helperGood1 string\n"`; `"-99999999999999999999"`, `"-18446744073709551616"` -> `""`; exit 0 | `err_08_scanf_erange_saturation` | [x] |
| 9 | `main` / `scanf("%d")` | **base-prefix rejection**: `%d` fixes the base at 10, so after a leading `'0'` the following `'x'`/`'X'` is neither consumed nor honored; the digit loop breaks on it and the value is just `0` (`"0x10"`, `"0X1F"`, `"0xy"`) | stdout `""`, exit 0 | `err_09_scanf_hex_prefix_not_honored` | [x] |
| 10 | `main` / `scanf("%d")` | **partial match then rejection**: digits followed by a non-digit terminate the conversion; the trailing garbage is pushed back and never examined (`"5abc"`, `"0nonsense"`, `"12."`) | value = the leading digits only; `"0nonsense"` -> `""`, `"5abc"` -> `"helperGood1 string\n"`; exit 0 | `err_10_scanf_partial_then_garbage` | [x] |
| 11 | `main` | `main` has no error return path at all — line 70 is the sole `return`, so the exit status is `0` for **every** input including all rejections above | exit code 0 | asserted in every row's test | [x] |

## Generic FFI boundary cases (covered even though not in the table)

| # | case | expected C result | test | [x] |
|---|------|-------------------|------|-----|
| G1 | `printLine(NULL)` — the null pointer | no output | `err_01_print_line_null` | [x] |
| G2 | `printLine("")` — zero-length (but non-null) string; `puts` still emits the newline | stdout `"\n"` | `err_g2_print_line_empty` | [x] |
| G3 | `printLine` with a very long (oversized, 1 MiB) string — no length limit exists in C | the bytes plus `"\n"` | `err_g3_print_line_oversized` | [x] |
| G4 | `printLine` with non-UTF-8 / high bytes (`0x80..0xFF`) and embedded control characters — `puts` is byte-transparent, so the Rust side must not attempt UTF-8 validation | the raw bytes plus `"\n"` | `err_g4_print_line_non_utf8` | [x] |
| G5 | `printLine` with a string containing embedded `"\n"` | the bytes plus one more `"\n"` | `err_g5_print_line_embedded_newline` | [x] |
| G6 | every single byte value `0x01..0xFF` as a one-character argument | that byte plus `"\n"` | `err_g6_print_line_every_byte` | [x] |
| G7 | one step past the `int` range in both directions: `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `4294967295`, `4294967296` | matches the `(int) num.l` truncation exactly | `err_g7_scanf_int_boundaries` | [x] |
| G8 | one step past the `long` range in both directions: `9223372036854775807`, `9223372036854775808`, `-9223372036854775808`, `-9223372036854775809` | matches `strtol` saturation + truncation | `err_g8_scanf_long_boundaries` | [x] |
| G9 | out-of-range "enum" values across the FFI boundary | **N/A by construction**: the C ABI surface is `void printLine(const char *)`, `void bad(void)`, `void good(void)`, `int main(void)`. There is no `enum`, no mode/flag parameter, and no integer selector anywhere in `c_src/src/main.c`, so there is no invalid-variant input to construct. Documented rather than tested. | — | [x] |
| G10 | zero-length stdin vs. stdin that is not readable | both take the `bad()` path, stdout `""`, exit 0 | `err_03_scanf_eof_empty` | [x] |
