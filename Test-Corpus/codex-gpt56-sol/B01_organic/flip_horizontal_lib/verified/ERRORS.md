# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
error enums, `assert`, explicit null/range checks, and min/max constants in
`c_src/include/*.h` and `c_src/src/*.c`.

The C API contains no rejection checks, error returns, asserts, enums, or
documented range constants. `flip_horizontal` returns `void` and assumes that
any memory it accesses through `img` and `img->pix` is valid. Consequently the
source-derived error-surface table has no rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Generic FFI boundary tests separately cover null pointers and extreme
dimensions. Inputs that make the C implementation perform invalid pointer
arithmetic or out-of-bounds memory access have undefined behavior and cannot
have a byte-identical portable result.

| Generic boundary | Differential coverage | Status |
|------------------|-----------------------|--------|
| Null `img` pointer | Isolated child processes compare terminating signals | [x] |
| Null `pix` pointer with accessed pixels | Isolated child processes compare terminating signals | [x] |
| Null `pix` pointer with zero work | Both shared libraries return normally | [x] |
| Zero dimensions | Covered by `CONFIGS.md` rows 1, 2, 6, and 11 | [x] |
| Extreme dimensions that perform zero work | `i32::MIN`/`i32::MAX` width and `i32::MIN` height | [x] |
| Out-of-range enum values | Not applicable: the public API has no enums | N/A |
| One past a documented range | Not applicable: the public API documents no ranges | N/A |
