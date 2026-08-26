# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or compile definitions. There is exactly one valid feature combination:

| # | Cargo feature combination | CMake configuration | Status |
|---|---------------------------|---------------------|--------|
| 1 | empty set (`--no-default-features`) | default | [x] |

## Runtime Configurations

The public API consists only of `hdr_compare`. It has no runtime option or mode
parameters. The rows below mechanically partition every predicate and
short-circuit branch in `hdr_valid` and `hdr_compare`. Each byte category is
sampled across many randomized values satisfying the stated constraints.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_compare` | `h2[0] != 0xff`; `h1` may be null because C short-circuits before reading it | [x] |
| 2 | `hdr_compare` | `h2[0] == 0xff`; `h2[1]` matches neither `(x & 0xf0) == 0xf0` nor `(x & 0xfe) == 0xe2` | [x] |
| 3 | `hdr_compare` | sync accepted by the `0xf0` high-nibble branch; version bits `((h2[1] >> 1) & 3) == 0` | [x] |
| 4 | `hdr_compare` | sync accepted by the `(x & 0xfe) == 0xe2` branch; remaining validity and comparison predicates satisfied | [x] |
| 5 | `hdr_compare` | sync/version valid; `h2[2] >> 4 == 15` | [x] |
| 6 | `hdr_compare` | sync/version/bitrate valid; `((h2[2] >> 2) & 3) == 3` | [x] |
| 7 | `hdr_compare` | valid `h2`; `((h1[1] ^ h2[1]) & 0xfe) != 0` | [x] |
| 8 | `hdr_compare` | valid `h2`; `h1[1]` differs from `h2[1]` only in bit 0; later predicates match | [x] |
| 9 | `hdr_compare` | valid `h2`; byte-1 predicate matches; `((h1[2] ^ h2[2]) & 0x0c) != 0` | [x] |
| 10 | `hdr_compare` | valid `h2`; byte-1/layer predicates match; both bitrate nibbles are nonzero (exact nibble values may differ) | [x] |
| 11 | `hdr_compare` | valid `h2` with nonzero bitrate nibble; byte-1/layer predicates match; `h1` bitrate nibble is zero | [x] |
| 12 | `hdr_compare` | valid `h2` with zero bitrate nibble; byte-1/layer predicates match; `h1` bitrate nibble is nonzero | [x] |
| 13 | `hdr_compare` | valid `h2`; byte-1/layer predicates match; both bitrate nibbles are zero | [x] |

Rows 1-3, 5-7, 9, 11, and 12 expect `0`. Rows 4, 8, 10, and 13
expect `1`.
