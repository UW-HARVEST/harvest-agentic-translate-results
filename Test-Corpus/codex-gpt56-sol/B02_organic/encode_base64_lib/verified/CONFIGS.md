# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
source selection. There is exactly one valid feature combination:

| # | Cargo feature combination | C configuration | checked |
|---|---------------------------|-----------------|---------|
| 1 | empty set (`--no-default-features`) | default CMake configuration | [x] |

## Runtime configurations

The sole public entry point is the lowest-level API. Rows below are the
cross-product portions distinguished by the C branches: explicit-size versus
`strlen` mode, zero/one/many loop iterations, remainder modulo three, signed
size behavior, byte values (including embedded NUL), and practical large
lengths. Randomized rows vary byte values across all four branches in the
private `encode` helper.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `encode_base64` | explicit `size = 1`; one block; one source byte and two padding characters | [x] |
| 2 | `encode_base64` | explicit `size = 2`; one block; two source bytes and one padding character | [x] |
| 3 | `encode_base64` | explicit `size = 3`; one complete block; no padding | [x] |
| 4 | `encode_base64` | explicit positive size; many blocks; `size % 3 == 1` | [x] |
| 5 | `encode_base64` | explicit positive size; many blocks; `size % 3 == 2` | [x] |
| 6 | `encode_base64` | explicit positive size; many complete blocks; `size % 3 == 0` | [x] |
| 7 | `encode_base64` | `size = 0`; empty NUL-terminated string; zero loop iterations | [x] |
| 8 | `encode_base64` | `size = 0`; `strlen(src) == 1`; one-byte remainder | [x] |
| 9 | `encode_base64` | `size = 0`; `strlen(src) == 2`; two-byte remainder | [x] |
| 10 | `encode_base64` | `size = 0`; many non-NUL bytes; effective length `% 3 == 1` | [x] |
| 11 | `encode_base64` | `size = 0`; many non-NUL bytes; effective length `% 3 == 2` | [x] |
| 12 | `encode_base64` | `size = 0`; many non-NUL bytes; effective length `% 3 == 0` | [x] |
| 13 | `encode_base64` | explicit size with embedded NUL bytes and bytes `0x80..0xff`; all byte values are data | [x] |
| 14 | `encode_base64` | explicit large readable inputs of lengths 65535, 65536, and 65537 | [x] |
| 15 | `encode_base64` | negative size whose small allocation succeeds; loop condition is initially false and result is empty | [x] |
