# Differential Mismatches

## Decimal overflow before `int` narrowing

- Input class: a choice or distance containing a decimal magnitude larger than
  the host's signed `long` range, such as `9223372036854775808`.
- Observed mismatch: C parsed the example as `-1` after saturation and
  narrowing, while Rust parsed it as `0`.
- Cause: Rust accumulated decimal digits modulo 32 bits. The C runtime's
  `sscanf("%d")` conversion saturates at the signed `long` boundary on this
  platform and then narrows the result to 32 bits.
- Fix: accumulate with saturation at `LONG_MAX` or `LONG_MIN` magnitude before
  applying 32-bit narrowing.
