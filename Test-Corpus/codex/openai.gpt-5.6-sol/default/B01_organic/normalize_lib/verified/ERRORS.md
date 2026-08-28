# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, `if`, `switch`, preprocessor conditionals, `NULL`, min/max constants,
and enums in `c_src/include` and `c_src/src`.

`normalize` returns `void` and contains no rejection, error return, assertion,
explicit range check, null check, enum, or min/max constant. Consequently, the
C rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Generic Boundary Coverage

These are boundary probes required by the verification protocol, not C
rejections. Cases whose C behavior is undefined are isolated in child
processes; differential coverage only requires the Rust and C processes to
terminate equivalently and never treats that behavior as a supported contract.

| # | boundary configuration | observed C behavior to match | Status |
|---|------------------------|------------------------------|--------|
| B1 | `dest = NULL`, `src = NULL`, `size = 0` | returns normally without access | [x] |
| B2 | `dest = NULL`, valid `src`, `size = 0` | zero-byte `memset`, then returns | [x] |
| B3 | valid `dest`, `src = NULL`, `size = 0` | zero-byte `memset`, then returns | [x] |
| B4 | `dest = NULL`, valid `src`, `size = 1` | no C rejection; process fault probe | [x] |
| B5 | valid `dest`, `src = NULL`, `size = 1` | no C rejection; process fault probe | [x] |
| B6 | aliased valid buffers, `size = -1` | loops are skipped and buffer is unchanged | [x] |
| B7 | distinct valid buffers, `size = -1` | huge signed-to-`size_t` `memset`; process fault probe | [x] |
| B8 | valid one-element buffers, `size = INT_MAX` | no C rejection; process fault probe | [x] |

There are no enum parameters and no documented upper bound, so enum
out-of-range and one-past-documented-range cases do not exist for this API.
