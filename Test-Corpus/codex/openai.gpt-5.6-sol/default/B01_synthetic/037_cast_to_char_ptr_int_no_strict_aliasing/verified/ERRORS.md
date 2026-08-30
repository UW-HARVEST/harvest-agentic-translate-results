# Differential Test Errors

No mismatches were found.

The initial Rust implementation matched the C executable for all audited input
classes:

- input failure: empty input and whitespace followed by EOF
- matching failure: invalid text and a sign without digits
- successful conversion: zero, positive, negative, `INT_MAX`, and `INT_MIN`
- out-of-range positive and negative decimal input
- leading newlines, trailing nonnumeric text, and multiple input items

Each comparison checks stdout bytes, stderr bytes, and exit status.
