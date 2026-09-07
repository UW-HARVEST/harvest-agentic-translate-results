# Configuration Surface

The sole public entry point is also the lowest-level entry point. For valid
inputs, the C source distinguishes:

- version index: bit 3 of `h[1]`, values 0 or 1;
- layer index: bits 2..1 of `h[1]`, selectors 1, 2, or 3;
- bitrate index: the high nibble of `h[2]`, values 0 through 14.

The table is their full valid cross-product. Each row's randomized differential
cases also vary all ignored input state: all of `h[0]`, bit 0 and bits 7..4 of
`h[1]`, the low nibble of `h[2]`, and trailing bytes after the required
three-byte header.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=0, buffer length >=3 | [x] |
| 2 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=1, buffer length >=3 | [x] |
| 3 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=2, buffer length >=3 | [x] |
| 4 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=3, buffer length >=3 | [x] |
| 5 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=4, buffer length >=3 | [x] |
| 6 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=5, buffer length >=3 | [x] |
| 7 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=6, buffer length >=3 | [x] |
| 8 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=7, buffer length >=3 | [x] |
| 9 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=8, buffer length >=3 | [x] |
| 10 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=9, buffer length >=3 | [x] |
| 11 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=10, buffer length >=3 | [x] |
| 12 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=11, buffer length >=3 | [x] |
| 13 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=12, buffer length >=3 | [x] |
| 14 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=13, buffer length >=3 | [x] |
| 15 | `hdr_bitrate` | version=0, layer_selector=1, bitrate_index=14, buffer length >=3 | [x] |
| 16 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=0, buffer length >=3 | [x] |
| 17 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=1, buffer length >=3 | [x] |
| 18 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=2, buffer length >=3 | [x] |
| 19 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=3, buffer length >=3 | [x] |
| 20 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=4, buffer length >=3 | [x] |
| 21 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=5, buffer length >=3 | [x] |
| 22 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=6, buffer length >=3 | [x] |
| 23 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=7, buffer length >=3 | [x] |
| 24 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=8, buffer length >=3 | [x] |
| 25 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=9, buffer length >=3 | [x] |
| 26 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=10, buffer length >=3 | [x] |
| 27 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=11, buffer length >=3 | [x] |
| 28 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=12, buffer length >=3 | [x] |
| 29 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=13, buffer length >=3 | [x] |
| 30 | `hdr_bitrate` | version=0, layer_selector=2, bitrate_index=14, buffer length >=3 | [x] |
| 31 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=0, buffer length >=3 | [x] |
| 32 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=1, buffer length >=3 | [x] |
| 33 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=2, buffer length >=3 | [x] |
| 34 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=3, buffer length >=3 | [x] |
| 35 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=4, buffer length >=3 | [x] |
| 36 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=5, buffer length >=3 | [x] |
| 37 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=6, buffer length >=3 | [x] |
| 38 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=7, buffer length >=3 | [x] |
| 39 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=8, buffer length >=3 | [x] |
| 40 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=9, buffer length >=3 | [x] |
| 41 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=10, buffer length >=3 | [x] |
| 42 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=11, buffer length >=3 | [x] |
| 43 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=12, buffer length >=3 | [x] |
| 44 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=13, buffer length >=3 | [x] |
| 45 | `hdr_bitrate` | version=0, layer_selector=3, bitrate_index=14, buffer length >=3 | [x] |
| 46 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=0, buffer length >=3 | [x] |
| 47 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=1, buffer length >=3 | [x] |
| 48 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=2, buffer length >=3 | [x] |
| 49 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=3, buffer length >=3 | [x] |
| 50 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=4, buffer length >=3 | [x] |
| 51 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=5, buffer length >=3 | [x] |
| 52 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=6, buffer length >=3 | [x] |
| 53 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=7, buffer length >=3 | [x] |
| 54 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=8, buffer length >=3 | [x] |
| 55 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=9, buffer length >=3 | [x] |
| 56 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=10, buffer length >=3 | [x] |
| 57 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=11, buffer length >=3 | [x] |
| 58 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=12, buffer length >=3 | [x] |
| 59 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=13, buffer length >=3 | [x] |
| 60 | `hdr_bitrate` | version=1, layer_selector=1, bitrate_index=14, buffer length >=3 | [x] |
| 61 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=0, buffer length >=3 | [x] |
| 62 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=1, buffer length >=3 | [x] |
| 63 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=2, buffer length >=3 | [x] |
| 64 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=3, buffer length >=3 | [x] |
| 65 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=4, buffer length >=3 | [x] |
| 66 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=5, buffer length >=3 | [x] |
| 67 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=6, buffer length >=3 | [x] |
| 68 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=7, buffer length >=3 | [x] |
| 69 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=8, buffer length >=3 | [x] |
| 70 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=9, buffer length >=3 | [x] |
| 71 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=10, buffer length >=3 | [x] |
| 72 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=11, buffer length >=3 | [x] |
| 73 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=12, buffer length >=3 | [x] |
| 74 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=13, buffer length >=3 | [x] |
| 75 | `hdr_bitrate` | version=1, layer_selector=2, bitrate_index=14, buffer length >=3 | [x] |
| 76 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=0, buffer length >=3 | [x] |
| 77 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=1, buffer length >=3 | [x] |
| 78 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=2, buffer length >=3 | [x] |
| 79 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=3, buffer length >=3 | [x] |
| 80 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=4, buffer length >=3 | [x] |
| 81 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=5, buffer length >=3 | [x] |
| 82 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=6, buffer length >=3 | [x] |
| 83 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=7, buffer length >=3 | [x] |
| 84 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=8, buffer length >=3 | [x] |
| 85 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=9, buffer length >=3 | [x] |
| 86 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=10, buffer length >=3 | [x] |
| 87 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=11, buffer length >=3 | [x] |
| 88 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=12, buffer length >=3 | [x] |
| 89 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=13, buffer length >=3 | [x] |
| 90 | `hdr_bitrate` | version=1, layer_selector=3, bitrate_index=14, buffer length >=3 | [x] |

Completion: [x] all 90 rows pass 512 fixed-seed randomized inputs each (46,080
valid differential calls per feature configuration).
