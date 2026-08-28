# Differential Mismatches

## Signed-long overflow while scanning timestamps

- Input timestamp: `18446744073709551616`
- C output timestamp: `4294967295`
- Initial Rust output timestamp: `0000000000`
- Cause: Rust accumulated decimal digits with wrapping `u32` arithmetic. On this
  platform, `scanf("%d")` parses through a signed 64-bit `long`, clamps positive
  overflow to `LONG_MAX`, and then stores the low 32 bits in the C
  `unsigned int`.
- Fix: timestamp parsing now saturates at `LONG_MAX` or the magnitude of
  `LONG_MIN` before applying the same 32-bit conversion as the C executable.
  The differential overflow cases pass.
