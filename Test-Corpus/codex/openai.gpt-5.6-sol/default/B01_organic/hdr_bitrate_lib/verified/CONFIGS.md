# Configuration Surface

The public API has one entry point and no runtime options or compile-time Cargo
features. Its defined behavior branches through a `2 x 3 x 15` lookup table:

- version row: boolean value of `h[1] & 0x08`;
- layer row: `((h[1] >> 1) & 3) - 1`, requiring encoded layer `1..=3`;
- bitrate column: `h[2] >> 4`, requiring index `0..=14`.

The low bit and high nibble of `h[1]`, the low nibble of `h[2]`, and all of
`h[0]` are ignored. Tests randomize those bits for every row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_bitrate` | version=0, layer=1, bitrate index=0, 3-byte header | [x] |
| 2 | `hdr_bitrate` | version=0, layer=1, bitrate index=1, 3-byte header | [x] |
| 3 | `hdr_bitrate` | version=0, layer=1, bitrate index=2, 3-byte header | [x] |
| 4 | `hdr_bitrate` | version=0, layer=1, bitrate index=3, 3-byte header | [x] |
| 5 | `hdr_bitrate` | version=0, layer=1, bitrate index=4, 3-byte header | [x] |
| 6 | `hdr_bitrate` | version=0, layer=1, bitrate index=5, 3-byte header | [x] |
| 7 | `hdr_bitrate` | version=0, layer=1, bitrate index=6, 3-byte header | [x] |
| 8 | `hdr_bitrate` | version=0, layer=1, bitrate index=7, 3-byte header | [x] |
| 9 | `hdr_bitrate` | version=0, layer=1, bitrate index=8, 3-byte header | [x] |
| 10 | `hdr_bitrate` | version=0, layer=1, bitrate index=9, 3-byte header | [x] |
| 11 | `hdr_bitrate` | version=0, layer=1, bitrate index=10, 3-byte header | [x] |
| 12 | `hdr_bitrate` | version=0, layer=1, bitrate index=11, 3-byte header | [x] |
| 13 | `hdr_bitrate` | version=0, layer=1, bitrate index=12, 3-byte header | [x] |
| 14 | `hdr_bitrate` | version=0, layer=1, bitrate index=13, 3-byte header | [x] |
| 15 | `hdr_bitrate` | version=0, layer=1, bitrate index=14, 3-byte header | [x] |
| 16 | `hdr_bitrate` | version=0, layer=2, bitrate index=0, 3-byte header | [x] |
| 17 | `hdr_bitrate` | version=0, layer=2, bitrate index=1, 3-byte header | [x] |
| 18 | `hdr_bitrate` | version=0, layer=2, bitrate index=2, 3-byte header | [x] |
| 19 | `hdr_bitrate` | version=0, layer=2, bitrate index=3, 3-byte header | [x] |
| 20 | `hdr_bitrate` | version=0, layer=2, bitrate index=4, 3-byte header | [x] |
| 21 | `hdr_bitrate` | version=0, layer=2, bitrate index=5, 3-byte header | [x] |
| 22 | `hdr_bitrate` | version=0, layer=2, bitrate index=6, 3-byte header | [x] |
| 23 | `hdr_bitrate` | version=0, layer=2, bitrate index=7, 3-byte header | [x] |
| 24 | `hdr_bitrate` | version=0, layer=2, bitrate index=8, 3-byte header | [x] |
| 25 | `hdr_bitrate` | version=0, layer=2, bitrate index=9, 3-byte header | [x] |
| 26 | `hdr_bitrate` | version=0, layer=2, bitrate index=10, 3-byte header | [x] |
| 27 | `hdr_bitrate` | version=0, layer=2, bitrate index=11, 3-byte header | [x] |
| 28 | `hdr_bitrate` | version=0, layer=2, bitrate index=12, 3-byte header | [x] |
| 29 | `hdr_bitrate` | version=0, layer=2, bitrate index=13, 3-byte header | [x] |
| 30 | `hdr_bitrate` | version=0, layer=2, bitrate index=14, 3-byte header | [x] |
| 31 | `hdr_bitrate` | version=0, layer=3, bitrate index=0, 3-byte header | [x] |
| 32 | `hdr_bitrate` | version=0, layer=3, bitrate index=1, 3-byte header | [x] |
| 33 | `hdr_bitrate` | version=0, layer=3, bitrate index=2, 3-byte header | [x] |
| 34 | `hdr_bitrate` | version=0, layer=3, bitrate index=3, 3-byte header | [x] |
| 35 | `hdr_bitrate` | version=0, layer=3, bitrate index=4, 3-byte header | [x] |
| 36 | `hdr_bitrate` | version=0, layer=3, bitrate index=5, 3-byte header | [x] |
| 37 | `hdr_bitrate` | version=0, layer=3, bitrate index=6, 3-byte header | [x] |
| 38 | `hdr_bitrate` | version=0, layer=3, bitrate index=7, 3-byte header | [x] |
| 39 | `hdr_bitrate` | version=0, layer=3, bitrate index=8, 3-byte header | [x] |
| 40 | `hdr_bitrate` | version=0, layer=3, bitrate index=9, 3-byte header | [x] |
| 41 | `hdr_bitrate` | version=0, layer=3, bitrate index=10, 3-byte header | [x] |
| 42 | `hdr_bitrate` | version=0, layer=3, bitrate index=11, 3-byte header | [x] |
| 43 | `hdr_bitrate` | version=0, layer=3, bitrate index=12, 3-byte header | [x] |
| 44 | `hdr_bitrate` | version=0, layer=3, bitrate index=13, 3-byte header | [x] |
| 45 | `hdr_bitrate` | version=0, layer=3, bitrate index=14, 3-byte header | [x] |
| 46 | `hdr_bitrate` | version=1, layer=1, bitrate index=0, 3-byte header | [x] |
| 47 | `hdr_bitrate` | version=1, layer=1, bitrate index=1, 3-byte header | [x] |
| 48 | `hdr_bitrate` | version=1, layer=1, bitrate index=2, 3-byte header | [x] |
| 49 | `hdr_bitrate` | version=1, layer=1, bitrate index=3, 3-byte header | [x] |
| 50 | `hdr_bitrate` | version=1, layer=1, bitrate index=4, 3-byte header | [x] |
| 51 | `hdr_bitrate` | version=1, layer=1, bitrate index=5, 3-byte header | [x] |
| 52 | `hdr_bitrate` | version=1, layer=1, bitrate index=6, 3-byte header | [x] |
| 53 | `hdr_bitrate` | version=1, layer=1, bitrate index=7, 3-byte header | [x] |
| 54 | `hdr_bitrate` | version=1, layer=1, bitrate index=8, 3-byte header | [x] |
| 55 | `hdr_bitrate` | version=1, layer=1, bitrate index=9, 3-byte header | [x] |
| 56 | `hdr_bitrate` | version=1, layer=1, bitrate index=10, 3-byte header | [x] |
| 57 | `hdr_bitrate` | version=1, layer=1, bitrate index=11, 3-byte header | [x] |
| 58 | `hdr_bitrate` | version=1, layer=1, bitrate index=12, 3-byte header | [x] |
| 59 | `hdr_bitrate` | version=1, layer=1, bitrate index=13, 3-byte header | [x] |
| 60 | `hdr_bitrate` | version=1, layer=1, bitrate index=14, 3-byte header | [x] |
| 61 | `hdr_bitrate` | version=1, layer=2, bitrate index=0, 3-byte header | [x] |
| 62 | `hdr_bitrate` | version=1, layer=2, bitrate index=1, 3-byte header | [x] |
| 63 | `hdr_bitrate` | version=1, layer=2, bitrate index=2, 3-byte header | [x] |
| 64 | `hdr_bitrate` | version=1, layer=2, bitrate index=3, 3-byte header | [x] |
| 65 | `hdr_bitrate` | version=1, layer=2, bitrate index=4, 3-byte header | [x] |
| 66 | `hdr_bitrate` | version=1, layer=2, bitrate index=5, 3-byte header | [x] |
| 67 | `hdr_bitrate` | version=1, layer=2, bitrate index=6, 3-byte header | [x] |
| 68 | `hdr_bitrate` | version=1, layer=2, bitrate index=7, 3-byte header | [x] |
| 69 | `hdr_bitrate` | version=1, layer=2, bitrate index=8, 3-byte header | [x] |
| 70 | `hdr_bitrate` | version=1, layer=2, bitrate index=9, 3-byte header | [x] |
| 71 | `hdr_bitrate` | version=1, layer=2, bitrate index=10, 3-byte header | [x] |
| 72 | `hdr_bitrate` | version=1, layer=2, bitrate index=11, 3-byte header | [x] |
| 73 | `hdr_bitrate` | version=1, layer=2, bitrate index=12, 3-byte header | [x] |
| 74 | `hdr_bitrate` | version=1, layer=2, bitrate index=13, 3-byte header | [x] |
| 75 | `hdr_bitrate` | version=1, layer=2, bitrate index=14, 3-byte header | [x] |
| 76 | `hdr_bitrate` | version=1, layer=3, bitrate index=0, 3-byte header | [x] |
| 77 | `hdr_bitrate` | version=1, layer=3, bitrate index=1, 3-byte header | [x] |
| 78 | `hdr_bitrate` | version=1, layer=3, bitrate index=2, 3-byte header | [x] |
| 79 | `hdr_bitrate` | version=1, layer=3, bitrate index=3, 3-byte header | [x] |
| 80 | `hdr_bitrate` | version=1, layer=3, bitrate index=4, 3-byte header | [x] |
| 81 | `hdr_bitrate` | version=1, layer=3, bitrate index=5, 3-byte header | [x] |
| 82 | `hdr_bitrate` | version=1, layer=3, bitrate index=6, 3-byte header | [x] |
| 83 | `hdr_bitrate` | version=1, layer=3, bitrate index=7, 3-byte header | [x] |
| 84 | `hdr_bitrate` | version=1, layer=3, bitrate index=8, 3-byte header | [x] |
| 85 | `hdr_bitrate` | version=1, layer=3, bitrate index=9, 3-byte header | [x] |
| 86 | `hdr_bitrate` | version=1, layer=3, bitrate index=10, 3-byte header | [x] |
| 87 | `hdr_bitrate` | version=1, layer=3, bitrate index=11, 3-byte header | [x] |
| 88 | `hdr_bitrate` | version=1, layer=3, bitrate index=12, 3-byte header | [x] |
| 89 | `hdr_bitrate` | version=1, layer=3, bitrate index=13, 3-byte header | [x] |
| 90 | `hdr_bitrate` | version=1, layer=3, bitrate index=14, 3-byte header | [x] |
