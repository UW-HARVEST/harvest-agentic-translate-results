# Configuration Surface

The only public entry point is `decode_base64`. It has no runtime options,
compile-time features, explicit length, element type, format, or byte-order
setting. Rows below come from the branches in `decode`, `is_base64`, and
`decode_base64`; together they cover each filtered-length shape, decode class,
filtering path, padding branch, and one/many-chunk shape.

| # | entry point(s) | configuration (options set + input shape) | Covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `decode_base64` | One quartet; uppercase alphabet characters (`A`-`Z`); filtered length `0 mod 4`; neither padding branch taken | [x] |
| 2 | `decode_base64` | One quartet mixing lowercase (`a`-`z`), digits (`0`-`9`), `+`, and `/`; all decode classes exercised | [x] |
| 3 | `decode_base64` | Exactly one retained character (`1 mod 4`); `c2`, `c3`, and `c4` default to `A` | [x] |
| 4 | `decode_base64` | Exactly two retained characters (`2 mod 4`); `c3` and `c4` default to `A` | [x] |
| 5 | `decode_base64` | Exactly three retained characters (`3 mod 4`); `c4` defaults to `A` | [x] |
| 6 | `decode_base64` | `=` in position 3 and non-`=` in position 4; second output byte suppressed, third emitted | [x] |
| 7 | `decode_base64` | Non-`=` in position 3 and `=` in position 4; second output byte emitted, third suppressed | [x] |
| 8 | `decode_base64` | `=` in positions 3 and 4; both conditional output bytes suppressed | [x] |
| 9 | `decode_base64` | `=` in position 1 or 2; retained and decoded through the default value `63`, without suppressing bytes | [x] |
| 10 | `decode_base64` | Invalid bytes mixed before, between, and after valid bytes; invalid bytes ignored | [x] |
| 11 | `decode_base64` | Nonempty source containing no retained Base64 characters; successful empty decoded result | [x] |
| 12 | `decode_base64` | Multiple quartets, including padding or an incomplete final quartet; loop executes many times | [x] |
| 13 | `decode_base64` | Valid input whose decoded bytes contain embedded NUL bytes; compare produced bytes rather than C-string length | [x] |
| 14 | `decode_base64` | Long NUL-terminated input, exercising the API's only oversized-input boundary | [x] |
