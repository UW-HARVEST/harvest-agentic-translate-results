# Error Surface

Mechanical source searches covered `return`, `RETURN_ERROR`, `assert`,
comparisons, null checks, and limit/min/max identifiers in `../c_src/src` and
`../c_src/include`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows: `gaussian_kernel` returns `void` and contains no rejection,
error return, assertion, explicit range check, or null check. In particular,
the `sum > 0.0f` condition selects normalization and is not an error path.

## Mandatory FFI boundary cases

These are not C rejection paths, but Phase C exercises them as required.

| # | boundary | expected C behavior | verified |
|---|----------|---------------------|----------|
| B1 | null `dest`, `size <= -2` (including `INT_MIN`) | returns without dereferencing `dest` | [x] |
| B2 | null `dest`, `size >= -1` | invalid memory access; compare isolated process termination | [x] |
| B3 | non-null `dest`, `size == 0` | writes one unnormalized element at `dest[0]` | [x] |
| B4 | non-null `dest`, `size == -1` | writes one unnormalized element at `dest[0]` | [x] |
| B5 | oversized positive `size == INT_MAX`, null `dest` | invalid memory access at the first write; compare isolated process termination | [x] |
| B6 | enum value outside its declared range | not applicable: the public API has no enum parameter | [x] |
| B7 | one past a documented numeric range | not applicable: the public header documents no numeric range | [x] |
