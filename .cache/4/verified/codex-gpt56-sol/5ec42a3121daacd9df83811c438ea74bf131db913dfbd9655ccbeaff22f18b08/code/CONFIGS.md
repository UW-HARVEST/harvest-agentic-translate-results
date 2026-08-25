# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section, and `c_src/CMakeLists.txt` has no
options or conditional sources. There is exactly one valid build-time
configuration:

| # | Cargo invocation | CMake configuration | |
|---|---|---|---|
| 1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime Configurations

The sole public entry point is the lowest-level API itself:
`unsigned hdr_bitrate(const uint8_t *h)`.

The C expression has three table axes:

- version: `!!(h[1] & 0x08)`, values 0 and 1;
- layer: `((h[1] >> 1) & 3) - 1`; valid encoded layer bits are 1, 2, and 3;
- bitrate index: `h[2] >> 4`; valid values are 0 through 14.

Their valid cross-product has 2 x 3 x 15 = 90 mechanically distinct rows.
Byte 0, bits 7..4 and bit 0 of byte 1, and the low nibble of byte 2 do not
affect the selected table entry; randomized tests vary all of them as noise.
Invalid index encodings are tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=0, header length=3 | [x] |
| 2 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=1, header length=3 | [x] |
| 3 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=2, header length=3 | [x] |
| 4 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=3, header length=3 | [x] |
| 5 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=4, header length=3 | [x] |
| 6 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=5, header length=3 | [x] |
| 7 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=6, header length=3 | [x] |
| 8 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=7, header length=3 | [x] |
| 9 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=8, header length=3 | [x] |
| 10 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=9, header length=3 | [x] |
| 11 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=10, header length=3 | [x] |
| 12 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=11, header length=3 | [x] |
| 13 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=12, header length=3 | [x] |
| 14 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=13, header length=3 | [x] |
| 15 | `hdr_bitrate` | version=0, layer bits=1, bitrate index=14, header length=3 | [x] |
| 16 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=0, header length=3 | [x] |
| 17 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=1, header length=3 | [x] |
| 18 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=2, header length=3 | [x] |
| 19 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=3, header length=3 | [x] |
| 20 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=4, header length=3 | [x] |
| 21 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=5, header length=3 | [x] |
| 22 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=6, header length=3 | [x] |
| 23 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=7, header length=3 | [x] |
| 24 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=8, header length=3 | [x] |
| 25 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=9, header length=3 | [x] |
| 26 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=10, header length=3 | [x] |
| 27 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=11, header length=3 | [x] |
| 28 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=12, header length=3 | [x] |
| 29 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=13, header length=3 | [x] |
| 30 | `hdr_bitrate` | version=0, layer bits=2, bitrate index=14, header length=3 | [x] |
| 31 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=0, header length=3 | [x] |
| 32 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=1, header length=3 | [x] |
| 33 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=2, header length=3 | [x] |
| 34 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=3, header length=3 | [x] |
| 35 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=4, header length=3 | [x] |
| 36 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=5, header length=3 | [x] |
| 37 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=6, header length=3 | [x] |
| 38 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=7, header length=3 | [x] |
| 39 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=8, header length=3 | [x] |
| 40 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=9, header length=3 | [x] |
| 41 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=10, header length=3 | [x] |
| 42 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=11, header length=3 | [x] |
| 43 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=12, header length=3 | [x] |
| 44 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=13, header length=3 | [x] |
| 45 | `hdr_bitrate` | version=0, layer bits=3, bitrate index=14, header length=3 | [x] |
| 46 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=0, header length=3 | [x] |
| 47 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=1, header length=3 | [x] |
| 48 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=2, header length=3 | [x] |
| 49 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=3, header length=3 | [x] |
| 50 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=4, header length=3 | [x] |
| 51 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=5, header length=3 | [x] |
| 52 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=6, header length=3 | [x] |
| 53 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=7, header length=3 | [x] |
| 54 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=8, header length=3 | [x] |
| 55 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=9, header length=3 | [x] |
| 56 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=10, header length=3 | [x] |
| 57 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=11, header length=3 | [x] |
| 58 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=12, header length=3 | [x] |
| 59 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=13, header length=3 | [x] |
| 60 | `hdr_bitrate` | version=1, layer bits=1, bitrate index=14, header length=3 | [x] |
| 61 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=0, header length=3 | [x] |
| 62 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=1, header length=3 | [x] |
| 63 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=2, header length=3 | [x] |
| 64 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=3, header length=3 | [x] |
| 65 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=4, header length=3 | [x] |
| 66 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=5, header length=3 | [x] |
| 67 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=6, header length=3 | [x] |
| 68 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=7, header length=3 | [x] |
| 69 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=8, header length=3 | [x] |
| 70 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=9, header length=3 | [x] |
| 71 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=10, header length=3 | [x] |
| 72 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=11, header length=3 | [x] |
| 73 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=12, header length=3 | [x] |
| 74 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=13, header length=3 | [x] |
| 75 | `hdr_bitrate` | version=1, layer bits=2, bitrate index=14, header length=3 | [x] |
| 76 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=0, header length=3 | [x] |
| 77 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=1, header length=3 | [x] |
| 78 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=2, header length=3 | [x] |
| 79 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=3, header length=3 | [x] |
| 80 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=4, header length=3 | [x] |
| 81 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=5, header length=3 | [x] |
| 82 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=6, header length=3 | [x] |
| 83 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=7, header length=3 | [x] |
| 84 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=8, header length=3 | [x] |
| 85 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=9, header length=3 | [x] |
| 86 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=10, header length=3 | [x] |
| 87 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=11, header length=3 | [x] |
| 88 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=12, header length=3 | [x] |
| 89 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=13, header length=3 | [x] |
| 90 | `hdr_bitrate` | version=1, layer bits=3, bitrate index=14, header length=3 | [x] |
