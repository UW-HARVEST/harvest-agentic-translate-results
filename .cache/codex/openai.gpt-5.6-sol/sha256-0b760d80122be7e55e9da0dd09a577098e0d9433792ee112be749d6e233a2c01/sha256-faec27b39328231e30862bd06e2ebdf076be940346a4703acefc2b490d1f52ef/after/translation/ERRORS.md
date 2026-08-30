# Differential Verification Errors

No C/Rust output mismatches were found.

Before adding the integration suite, 1,026 subprocess comparisons covered the
integer bounds, each `strtol` failure condition, embedded NUL bytes, `fgets`
newline and 99-byte limits, and deterministic random byte inputs. Each
comparison included stdout, stderr, and exit status.

The audited input classes are encoded in `tests/differential.rs`:

- successful values: zero, positive, negative, `INT_MAX`, and `INT_MIN`
- accepted prefixes: leading whitespace/signs and trailing non-digits
- no conversion: EOF, blank/whitespace, nonnumeric input, and bare signs
- range failures: values outside `int` and values overflowing `long`
- input boundaries: newline stopping, embedded NUL, no final newline, and
  truncation at the 99-byte `fgets` payload limit
