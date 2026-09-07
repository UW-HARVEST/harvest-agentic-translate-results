# Error surface

The public ABI is `void rgb_to_hsv(float *dest, const float *src)`. The C
implementation contains no error-return statements, error macros, assertions,
range checks, enum arguments, length arguments, or null checks. Consequently,
it has no recoverable error result. Its only invalid-input behavior is the
unchecked pointer dereference below, which is compared out of process because
it terminates the caller.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `rgb_to_hsv` | `src == NULL` | process receives a memory-access fault while reading `src[0]` | [x] |
| 2 | `rgb_to_hsv` | `dest == NULL` with a valid non-null three-float `src` | process receives a memory-access fault while writing `dest[0]` | [x] |

Generic boundary applicability:

- Zero and oversized lengths: not applicable; the API has no length argument.
- One-past-range values: not applicable; the header documents no numeric range.
- Out-of-range enum values: not applicable; the API has no enum argument.
