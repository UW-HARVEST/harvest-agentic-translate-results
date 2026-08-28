# Differential Verification Errors

## Mismatches

No mismatches were found. The existing Rust implementation matched the C
executable for stdout bytes, stderr bytes, and exit status in every enumerated
case, so no Rust source correction was required.

## Audited Input Classes

- Argument count errors: zero operands, one operand, and more than two operands.
- Successful parsing and calculation: finite integers, fractional values,
  leading whitespace, the maximum finite `double`, signed zero, infinity, NaN,
  and two-decimal formatting.
- Base conversion errors: invalid or partially valid text, overflow, underflow,
  trailing whitespace, and range-error precedence over trailing junk.
- Exponent conversion errors: invalid text, overflow, underflow, and
  range-error precedence over trailing junk.
- Validation order: a base error is reported before an exponent error.
- `pow` errors: domain error, overflow, underflow, and pole error.
- C-specific edge behavior: empty and whitespace-only numeric strings are
  accepted as zero because the C code only checks the byte at `endptr`.
