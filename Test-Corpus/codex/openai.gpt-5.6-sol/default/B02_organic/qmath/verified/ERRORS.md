# Differential Verification Errors

No mismatches were found.

The subprocess suite compares stdout, stderr, and exit status for:

- zero, one, two, and four command-line items (all reachable error-count
  classes);
- exactly three items, the maximum accepted count;
- regular positive, negative, and decimal values;
- zero vectors, signed zero, empty strings, nonnumeric strings, numeric
  prefixes, and leading whitespace as handled by `atof`;
- finite extremes, overflow, underflow, subnormal values, infinities, and NaN.

The Phase C call-path audit found no additional input-dependent branch:
`VectorNormalizeFast` intentionally does not check for a zero length, and
`Q_rsqrt` has no conditional or early return.
