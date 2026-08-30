# Differential Mismatches

This file records mismatches found while comparing `c_src/build/driver` with
the Rust `driver` executable.

## Mismatches

No mismatches were observed.

## Input Coverage

- Empty input: both `fgets` calls return `NULL`.
- One line: the first `fgets` succeeds and the second returns `NULL`.
- Two lines, including a final line without `\n`: both reads succeed.
- Blank, zero, negative-zero, invalid, NaN, and infinite values: `atof`,
  unordered comparison, and zero-guard behavior.
- Values immediately around `0.000001`: both outcomes of the `fabs` guard.
- Positive and negative fractional results: integer truncation and signedness.
- Tiny positive and negative divisors: out-of-range float-to-`int` conversion.
- Largest finite `float`: finite range boundary.
- 18 bytes plus newline, 19 bytes at EOF, and more than 19 bytes: every
  `fgets` buffer-boundary behavior, including data spilling into the next read.
- Embedded NUL: `fgets` continues reading while `atof` stops at the NUL.

The `line != NULL` false branch in `printLine` is not reachable from `main`;
every call passes a string literal. Command-line arguments are ignored.
