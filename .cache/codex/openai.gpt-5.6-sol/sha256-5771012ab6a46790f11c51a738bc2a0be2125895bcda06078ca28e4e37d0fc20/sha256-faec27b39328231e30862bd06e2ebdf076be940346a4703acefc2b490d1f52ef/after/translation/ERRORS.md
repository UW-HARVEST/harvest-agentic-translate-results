# Differential Test Mismatches

No mismatches were found.

The initial Rust executable matched the C reference for stdout, stderr, and
exit status across all enumerated input classes.

## Input classes checked

- Empty input and whitespace-only input, where `scanf` reaches end of input
  without assigning `x`.
- A malformed leading token, where `scanf` fails without assigning `x`.
- Parsed zero.
- Positive and negative nonzero integers.
- `INT_MAX` and `INT_MIN`.
- Positive and negative decimal values immediately outside the `int` range.
- Leading whitespace, an explicit plus sign, and an integer following a
  newline, exercising `scanf` whitespace and token handling.
- A valid integer followed by trailing nonnumeric text.

The C program has no explicit error return or diagnostic branch. Every path
returns status 0 and writes nothing to stderr.

## Branch audit

Only `if (x)` is controlled by stdin, and the suite reaches both outcomes.
The remaining conditions operate on fixed values:

- `printLine` receives only a non-null string literal.
- `bad` always sets `data` to positive `CHAR_MAX`.
- `goodG2B` always sets `data` to positive `2`.
- `goodB2G` always sets `data` to positive `CHAR_MAX`, which always selects
  its "too large" branch.

Their opposite outcomes cannot be reached by any program input.
