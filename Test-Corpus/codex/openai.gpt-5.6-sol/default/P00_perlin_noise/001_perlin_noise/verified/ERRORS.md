# Differential Verification

## Mismatches

No mismatches were found.

## Coverage

The subprocess differential tests compare stdout, stderr, and exit status for:

- Empty input, every partial `scanf` prefix, malformed input at every conversion,
  newline-separated input, and trailing input.
- Every `which` switch outcome: `0` through `5` and the default branch.
- Positive, negative non-integer, and negative integer coordinates.
- Zero, minimum, non-power-of-two, and maximum supported (`256`) wraps.
- Seed values that exercise unsigned-byte truncation.
- Negative, zero, one, multiple, and 256-octave loop boundaries.
- Negative-coordinate correction and upper-bound rollover in non-power-of-two
  wrapping.

The C driver has no explicit error-status path. Failed or incomplete `scanf`
conversions leave the remaining zero-initialized values unchanged, and the
program still prints one result and exits successfully; the tests preserve and
compare that behavior.

## Executables

- C: `../c_src/build/driver`
- Rust debug: `target/debug/driver`
- Rust release: `target/release/driver`
