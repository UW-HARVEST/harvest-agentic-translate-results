# Differential Verification Errors

No mismatches were found. The existing Rust translation matched the C
executable for stdout, stderr, and exit status in every differential case, so
`src/main.rs` required no changes.

## Audited Input Classes

- Empty and whitespace-only input, leaving both initialized values unchanged.
- Invalid first conversion, one successful conversion, and invalid second
  conversion.
- Two successful conversions on one line and across separate lines.
- Positive and negative operands, exact division, and nonzero remainders.
- `INT_MAX`, `INT_MIN`, and values one beyond each signed 32-bit boundary.
- Extra input after the two conversions.
- A zero denominator and `INT_MIN / -1`, both of which terminate via `SIGFPE`.

The C source contains no explicit conditional, null check, length branch, or
early return. Its sole `return` is the successful return at the end of `main`.
