# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

Grep used to enumerate every rejection point (result reproduced below):

```sh
grep -n 'return\|NULL\|assert\|exit(\|abort\|errno\|ERROR\|<\|>\|==\|!=\|if\|switch\|#if\|MAX\|MIN\|enum' \
     src/driver.c include/driver.h
```

Non-preprocessor hits in the whole library: **one**.

```
src/driver.c:31:    if(line != NULL)
```

Facts that constrain this table:

* Every public function returns `void`. There is **no** error code, sentinel
  return, out-parameter status, or error enum anywhere in the library.
* There is **no** `assert`, `exit`, `abort`, `errno` use, no numeric range
  check, and no `MIN`/`MAX` constant.
* There is **no** `enum` type in the API, so there is no "out-of-range enum
  value" input to construct (see row 5 for how that is discharged).
* `bad`, `good` and `driver` take no arguments, so their only observable
  contract is the exact byte stream they write to `stdout`.

Consequently the *only* input rejection the C performs is the null check in
`printLine`. Rows 2–6 are the generic FFI-boundary boundaries that Phase C
mandates regardless of the table; they are recorded here so each has a test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` (`if(line != NULL)` at `driver.c:31` is false) | returns normally, writes **0 bytes** to `stdout`; no crash | `err_row1_print_line_null` | [x] |
| 2 | `printLine` | `line` points at an empty string `""` (zero-length payload, valid but degenerate) | writes exactly one byte, `"\n"` | `err_row2_print_line_empty` | [x] |
| 3 | `printLine` | `line` points at a very long string (oversized length: 1 byte … 1 MiB, incl. lengths straddling any internal buffer size) | writes the whole payload verbatim plus `"\n"`; no truncation, no crash | `err_row3_print_line_oversized` | [x] |
| 4 | `printLine` | `line` contains bytes that are **not** valid UTF-8 (`0x80..=0xFF`) and bytes that look like `printf` format directives (`%s`, `%n`, `%d`) — the payload is passed as the *argument* of `"%s"`, never as the format string | bytes are copied through verbatim, `%` is **not** interpreted; then `"\n"` | `err_row4_print_line_non_utf8_and_format_chars` | [x] |
| 5 | `printIntLine` | values one step past / at the extremes of the `int` range: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`. This also discharges the "out-of-range enum value across the FFI boundary" case: the ABI-level parameter type is a plain 32-bit `int`, every bit pattern is a legal input, and `INT_MIN`/`INT_MAX` are the values with no "valid variant" | decimal rendering by `printf("%d\n", …)`; in particular `INT_MIN` prints `-2147483648` (no overflow, no UB from negation) | `err_row5_print_int_line_extremes` | [x] |
| 6 | `printLine` / `printIntLine` | called *repeatedly* and interleaved, including a `NULL` call between two valid calls (checks the rejected call leaves no residue in the `stdout` stream) | output is the concatenation of only the non-rejected calls | `err_row6_interleaved_with_rejections` | [x] |

## Deliberately NOT tested

Passing a non-null but **invalid / dangling** `const char *`, or a pointer to a
buffer with no NUL terminator, is undefined behaviour in the C (`printf("%s")`
walks until it finds a NUL). The C library performs no check for it, so there
is no defined result to compare against and a differential test would compare
two undefined behaviours. Row 3 covers the well-defined "very large, properly
terminated" end of that axis instead.
