# Error And Rejection Surface

The source contains no `assert`, error macro, error enum, null check, range
check, or explicit invalid-input sentinel. It has two ways to reject a match.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `match` | `total(test, bins) < threshold * total(reference, bins)` | `0` from the early return; neither input buffer is modified | [x] |
| 2 | `match` | Row 1 is false, then `spectral_contrast(t, r, bins) >= threshold` is false or unordered | `0` from the final comparison; neither input buffer is modified | [x] |

## Unchecked FFI Boundaries

These are not rejection rows because the C source does not validate them:

- `spectral_contrast` with `length <= 0` executes no element access and returns
  `0.0`; null pointers are therefore tolerated for those lengths.
- `spectral_contrast` with a positive length requires both pointers to address
  at least `length * sizeof(float)` readable and writable bytes.
- `match` requires `bins > 0` and both pointers to address at least
  `bins * sizeof(double)` readable bytes. With `bins <= 0`, its VLA and
  `v[length - 1]` access have undefined behavior.
- Neither function imposes a source-level maximum length. The practical maximum
  is constrained by addressable memory; `match` additionally allocates two
  variable-length arrays on the C stack.
- The API declares no enums, so there is no out-of-range enum surface.
