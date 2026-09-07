# Error and rejection surface

The library has no error enum, error macro, assertion, length parameter, or
documented min/max constant. Its public function is a predicate: each failed
condition below is a distinct rejection returning integer `0`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `hdr_compare` via `hdr_valid(h2)` | `h2[0] != 0xff` | `0` | [x] |
| 2 | `hdr_compare` via `hdr_valid(h2)` | `(h2[1] & 0xf0) != 0xf0 && (h2[1] & 0xfe) != 0xe2` after row 1 passes | `0` | [x] |
| 3 | `hdr_compare` via `hdr_valid(h2)` | `((h2[1] >> 1) & 3) == 0` after rows 1–2 pass | `0` | [x] |
| 4 | `hdr_compare` via `hdr_valid(h2)` | `(h2[2] >> 4) == 15` after rows 1–3 pass | `0` | [x] |
| 5 | `hdr_compare` via `hdr_valid(h2)` | `((h2[2] >> 2) & 3) == 3` after rows 1–4 pass | `0` | [x] |
| 6 | `hdr_compare` | `((h1[1] ^ h2[1]) & 0xfe) != 0` with valid `h2` | `0` | [x] |
| 7 | `hdr_compare` | `((h1[2] ^ h2[2]) & 0x0c) != 0` with valid `h2` and row 6 passing | `0` | [x] |
| 8 | `hdr_compare` | `((h1[2] & 0xf0) == 0) != ((h2[2] & 0xf0) == 0)` with valid `h2` and rows 6–7 passing | `0` | [x] |

## Generic FFI boundaries

There are no length or enum arguments, so zero/oversized lengths and
out-of-range enum values are not applicable. The C source performs no null
checks; tests therefore compare the observable process-level behavior.

| # | condition | expected C behavior | Verified |
|---|-----------|---------------------|----------|
| G1 | `h2 == NULL` | process receives `SIGSEGV` on dereference | [x] |
| G2 | `h1 == NULL` and `h2` is valid | process receives `SIGSEGV` when `h1[1]` is read | [x] |
| G3 | `h1 == NULL` and `h2` is rejected at its first byte | returns `0` without reading `h1` | [x] |
