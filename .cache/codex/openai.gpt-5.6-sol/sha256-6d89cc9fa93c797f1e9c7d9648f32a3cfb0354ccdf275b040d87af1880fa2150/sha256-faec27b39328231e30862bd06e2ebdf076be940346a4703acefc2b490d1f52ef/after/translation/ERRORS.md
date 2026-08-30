# Differential Verification Errors

No mismatches were found. The initial Rust executable matched the C executable
for stdout, stderr, and exit status across every tested input class, so no Rust
production-code changes were required.

## Audited Input Classes

- Input failure: empty input and whitespace followed by EOF.
- Matching failure: invalid text and an embedded NUL before a number.
- Successful conversion: one item, signed zero, finite positive and negative
  values, maximum finite `double`, minimum normal, and minimum subnormal.
- Range conversion: positive and negative overflow and underflow.
- Special and alternate forms: infinities, NaN, and hexadecimal floating point.
- Stream behavior: leading whitespace across lines, a partial numeric token,
  and a second item after the converted value.

The C source has no conditional statements, length checks, null checks, or
early returns. `scanf` failure leaves the initialized value at positive zero,
then `driver` always prints once and `main` returns zero.
