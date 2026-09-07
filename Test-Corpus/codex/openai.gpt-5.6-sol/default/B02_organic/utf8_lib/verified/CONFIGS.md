# Configuration surface

There are no Cargo features, compile-time C options, enums, length parameters,
or executable targets. The public dynamic surface consists of the two symbols
listed below. Rows are the reachable cross-product of entry point, the
`replacement` runtime option, and input shapes distinguished by the C
branches.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `w_utf8_drop`, `w_utf8_filter` | empty NUL-terminated string; filter option is not inspected on the all-valid fast path | [x] |
| 2 | `w_utf8_drop`, `w_utf8_filter` | one or many ASCII bytes (`0x01..0x7F`); all-valid fast path | [x] |
| 3 | `w_utf8_drop`, `w_utf8_filter` | valid two-byte sequences, including lower lead boundary `C2 80` and upper lead boundary `DF BF` | [x] |
| 4 | `w_utf8_drop`, `w_utf8_filter` | valid ordinary three-byte sequences with lead `E1..EC` or `EE`; continuation boundaries included | [x] |
| 5 | `w_utf8_drop`, `w_utf8_filter` | valid `E0 A0..BF 80..BF`, including the anti-overlong lower boundary | [x] |
| 6 | `w_utf8_drop`, `w_utf8_filter` | valid `ED 80..9F 80..BF`, including the pre-surrogate upper boundary | [x] |
| 7 | `w_utf8_drop`, `w_utf8_filter` | valid `EF 80..BF 80..BF`, including the explicit `EF` second-byte `BF` upper boundary | [x] |
| 8 | `w_utf8_drop`, `w_utf8_filter` | valid ordinary four-byte sequences with lead `F1..F3`; continuation boundaries included | [x] |
| 9 | `w_utf8_drop`, `w_utf8_filter` | valid `F0 90..BF 80..BF 80..BF`, including the anti-overlong lower boundary | [x] |
| 10 | `w_utf8_drop`, `w_utf8_filter` | valid `F4 80..8F 80..BF 80..BF`, including U+10FFFF upper boundary | [x] |
| 11 | `w_utf8_drop`, `w_utf8_filter` | mixed valid string containing ASCII plus two-, three-, and four-byte sequences; empty/one/many sequence counts randomized | [x] |
| 12 | `w_utf8_drop`, `w_utf8_filter` | valid prefix followed by one malformed byte/sequence from each rejection class | [x] |
| 13 | `w_utf8_filter` | malformed input, `replacement == false`; invalid bytes are deleted one at a time | [x] |
| 14 | `w_utf8_filter` | malformed input, `replacement == true`; each invalid byte becomes `EF BF BD` | [x] |
| 15 | `w_utf8_filter` | alternating valid and malformed spans, `replacement == false` | [x] |
| 16 | `w_utf8_filter` | alternating valid and malformed spans, `replacement == true` | [x] |
| 17 | `w_utf8_filter` | long malformed input with enough replacements to cross multiple `REPLACEMENT_INC == 4096` growth boundaries | [x] |
| 18 | `w_utf8_drop`, `w_utf8_filter` | long all-valid input (oversized practical boundary; no explicit length argument exists) | [x] |
