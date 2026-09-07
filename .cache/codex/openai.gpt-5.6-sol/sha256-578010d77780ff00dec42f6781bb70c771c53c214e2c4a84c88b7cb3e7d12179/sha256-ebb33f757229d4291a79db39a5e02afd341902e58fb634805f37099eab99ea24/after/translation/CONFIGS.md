# Configuration-surface table

The public header exposes one entry point and no runtime option setters, modes,
flags, enums, or compile-time features. Rows below come from the branches in
`encode_base64`: exact-zero size selects `strlen`; the loop distinguishes
zero/one/many iterations and final groups containing 1, 2, or 3 bytes; explicit
sizes can include NUL and high-bit bytes; and `encode` distinguishes sextet
ranges `0..25`, `26..51`, `52..61`, `62`, and `63`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `encode_base64` | explicit negative size `-1..=-3`; non-null source; loop executes zero times and allocation succeeds | [x] |
| 2 | `encode_base64` | inferred size (`size == 0`), empty C string (`strlen == 0`) | [x] |
| 3 | `encode_base64` | inferred size, non-empty C-string length `len % 3 == 1` | [x] |
| 4 | `encode_base64` | inferred size, non-empty C-string length `len % 3 == 2` | [x] |
| 5 | `encode_base64` | inferred size, non-empty C-string length `len % 3 == 0` | [x] |
| 6 | `encode_base64` | explicit positive size, `size % 3 == 1` (one-byte padded final group) | [x] |
| 7 | `encode_base64` | explicit positive size, `size % 3 == 2` (two-byte padded final group) | [x] |
| 8 | `encode_base64` | explicit positive size, `size % 3 == 0` (un-padded final group) | [x] |
| 9 | `encode_base64` | explicit size includes embedded NUL bytes, so bytes after NUL are encoded | [x] |
| 10 | `encode_base64` | explicit size is shorter than the backing input; only the prefix is encoded | [x] |
| 11 | `encode_base64` | explicit size with bytes `0x80..0xff` (signed-`char` source values converted to `unsigned char`) | [x] |
| 12 | `encode_base64` | inputs whose generated sextets cover all five `encode` result branches, including exact values 62 (`+`) and 63 (`/`) | [x] |
