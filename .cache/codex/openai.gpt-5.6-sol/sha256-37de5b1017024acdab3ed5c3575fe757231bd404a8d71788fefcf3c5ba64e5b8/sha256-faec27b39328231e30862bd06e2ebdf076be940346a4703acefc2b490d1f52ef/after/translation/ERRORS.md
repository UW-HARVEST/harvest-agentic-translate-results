# Differential Error Log

## Mismatches

No C/Rust mismatches were found. The existing Rust implementation matched the
C executable for stdout, stderr, and exit status in every enumerated case.

## Audited Input Classes

- End of input before a conversion: empty input and whitespace-only input.
- Failed conversion: invalid text, a sign without digits, and an initial NUL.
- Successful zero conversion: zero, first-item zero, and values that truncate
  to zero when stored as the C `int`.
- Successful nonzero conversion: positive and negative values, `INT_MAX`,
  `INT_MIN`, values outside the `int` range, and values outside the `long`
  range.
- Scanner boundaries: whitespace across newlines, a numeric prefix followed by
  text, multiple items, and input without a trailing newline.

For this C build, every input that leaves `x` equal to zero enters `bad()` and
prints exactly one newline. Nonzero input enters `good()` and prints
`string\n`. Both paths write nothing to stderr and exit with status zero.
