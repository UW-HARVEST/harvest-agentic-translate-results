# Error-Surface Table

Mechanically derived from every rejecting branch and boundary check in
`c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `wcscat` | `dst == NULL` with nonzero `numElem` | return `22`; no destination write | [x] |
| 2 | `wcscat` | `numElem == 0` with non-null `dst` | return `22`; no destination write | [x] |
| 3 | `wcscat` | `src == NULL` after passing the destination/length check | set `dst[0] = 0`, then return `22` | [x] |
| 4 | `wcscat` | destination has no NUL element in the first `numElem` elements, so the scan reaches `dst + numElem` | set `dst[0] = 0`, then return `34` | [x] |
| 5 | `wcscat` | destination is terminated, but no source NUL is copied before remaining destination capacity is exhausted | copy exactly the remaining source elements, set `dst[0] = 0`, then return `34` | [x] |
| 6 | `wcscat` | generic oversized-length boundary: `dst == NULL` and `numElem == SIZE_MAX` (the null check short-circuits before pointer arithmetic) | return `22`; no destination write | [x] |

There are no enums, assertions, error enums, option ranges, documented numeric
maxima, or other public functions in the C source.
