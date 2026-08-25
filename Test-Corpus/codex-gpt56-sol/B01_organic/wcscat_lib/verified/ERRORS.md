# Error Surface

Derived from every rejecting return and its controlling condition in
`c_src/src/lib.c`. `wcscat` has no enum parameters or documented maximum
length; `size_t` is the full accepted length type.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `wcscat` | `dst == NULL` with `numElem > 0` | return `22`; no destination exists to modify [x] |
| 2 | `wcscat` | `numElem == 0` with non-null `dst` | return `22`; destination remains unchanged [x] |
| 3 | `wcscat` | `dst == NULL` and `numElem == 0` | return `22`; the first compound rejection branch wins [x] |
| 4 | `wcscat` | `src == NULL` with non-null `dst` and `numElem > 0` | set `dst[0] = 0`; return `22` [x] |
| 5 | `wcscat` | no zero element occurs in `dst[0..numElem]` | set `dst[0] = 0`; return `34`; other destination elements remain unchanged [x] |
| 6 | `wcscat` | destination has a terminator, but no source terminator is copied before `ptr == dst + numElem` | set `dst[0] = 0`; return `34`; retain source elements copied before exhaustion [x] |

No assertions, error enums, min/max constants, range checks beyond
`numElem == 0`, or out-of-range enum cases exist in the C API.
