# Configuration Surface

The public API has two entry points and no compile-time Cargo features or C
feature macros. `spectral_contrast.c` includes `<math.h>` without `match.h`;
on this build platform its `float_t` is four bytes. Consequently direct calls
declared by `match.h` pass `double *`, but the implementation mutates the first
`length` four-byte float slots. The tests preserve and compare that behavior.

For `match`, the source distinguishes the total gate, the final contrast gate,
and count shapes around the one-element differentiation boundary and the
`N_SMOOTH == 16` smoothing boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `spectral_contrast` | `length == 0`; no elements, including null pointers | [x] |
| 2 | `spectral_contrast` | `length == 1`; finite nonzero float-slot values | [x] |
| 3 | `spectral_contrast` | `length > 1`; finite nonzero magnitudes, distinct buffers | [x] |
| 4 | `spectral_contrast` | `length > 1`; at least one zero-magnitude input | [x] |
| 5 | `spectral_contrast` | `length > 1`; `a == b` aliasing | [x] |
| 6 | `match` | `bins == 1`; total gate rejects | [x] |
| 7 | `match` | `bins == 1`; total gate passes, final contrast rejects | [x] |
| 8 | `match` | `2 <= bins < 16`; total gate rejects | [x] |
| 9 | `match` | `2 <= bins < 16`; total gate passes, final contrast rejects | [x] |
| 10 | `match` | `2 <= bins < 16`; both gates pass | [x] |
| 11 | `match` | `bins == 16`; total gate rejects | [x] |
| 12 | `match` | `bins == 16`; total gate passes, final contrast rejects | [x] |
| 13 | `match` | `bins == 16`; both gates pass | [x] |
| 14 | `match` | `bins > 16`; total gate rejects | [x] |
| 15 | `match` | `bins > 16`; total gate passes, final contrast rejects | [x] |
| 16 | `match` | `bins > 16`; both gates pass | [x] |
| 17 | `match` | positive `bins`; aliased `test == reference` | [x] |
| 18 | `match` | positive `bins`; unordered `NaN` threshold comparisons | [x] |

Each non-boundary row is exercised with many fixed-seed randomized inputs.
Rows describing an output path use generated candidates filtered by the C
result and, where needed, by a direct reconstruction of the total gate.
