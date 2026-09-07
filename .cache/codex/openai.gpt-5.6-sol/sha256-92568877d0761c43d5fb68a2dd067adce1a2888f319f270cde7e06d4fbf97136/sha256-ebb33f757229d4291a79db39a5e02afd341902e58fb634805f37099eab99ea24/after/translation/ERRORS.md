# Error surface

Mechanical search covered `c_src/src/lib.c` and `c_src/include/lib.h` for
error returns, `assert`, `if`, `switch`, null checks, range checks, enums, and
min/max constants.

The C source has no explicit rejection or error path, so the required
error-surface table has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|---|---|---|---|

The public function returns `void` and unconditionally dereferences `img`.
Consequently, invalid pointers are undefined behavior rather than a returned
error code or sentinel.

## Generic FFI boundary obligations

These are required even though they are not explicit C rejection branches:

| # | boundary case | expected C behavior | tested |
|---|---|---|---|
| G1 | `img == NULL` | Process terminates from invalid dereference; no error sentinel exists | [x] |
| G2 | `img->pix == NULL` with a positive pixel count | Process terminates from invalid dereference; no error sentinel exists | [x] |
| G3 | zero pixel count (`w == 0` or `h == 0`) | Returns normally without reading `pix` | [x] |
| G4 | negative loop bound (exactly one of `w`, `h` is negative) | Returns normally without reading `pix` | [x] |
| G5 | oversized but non-overflowing dimensions with zero height (`w == INT_MAX / 4`, `h == 0`) | Returns normally without reading `pix` | [x] |
| G6 | out-of-range enum value | Not applicable: the public API has no enum parameter | [x] |
| G7 | one step past documented valid range | Not applicable: the C header documents no numeric range | [x] |

Dimension combinations whose signed `int` multiplications overflow are C
undefined behavior and therefore do not have a stable ground-truth result to
compare.

G1 and G2 are compared in isolated subprocesses, including the exact
termination signal. G3 through G5 are differential FFI calls. G6 and G7 are
closed by mechanical inspection of the complete public header.
