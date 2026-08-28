# Configuration surface

The sole public entry point is `read_side_info`; `get_bits` is static. There
are no Cargo features. The rows below are the mechanically derived cross
product of all valid MPEG header profiles, channel/granule branches, and
coding-shape branches in `lib.c`.

Header profiles:

- P1: version bits `00`, sample-rate selector 0, `sr_idx=0`
- P2: version bits `00`, sample-rate selector 1, `sr_idx=0`
- P3: version bits `00`, sample-rate selector 2, `sr_idx=1`
- P4: version bits `10`, sample-rate selector 0, `sr_idx=2`
- P5: version bits `10`, sample-rate selector 1, `sr_idx=3`
- P6: version bits `10`, sample-rate selector 2, `sr_idx=4`
- P7: version bits `11`, sample-rate selector 0, `sr_idx=5`
- P8: version bits `11`, sample-rate selector 1, `sr_idx=6`
- P9: version bits `11`, sample-rate selector 2, `sr_idx=7`

Channel profiles:

- M: `hdr[3] & 0xC0 == 0xC0` (mono): 1 granule for P1-P6, 2 for P7-P9
- S: `hdr[3] & 0xC0 != 0xC0` (non-mono): 2 granules for P1-P6, 4 for P7-P9

Coding shapes:

- N: window-switching flag 0 (normal long block)
- W1: flag 1, block type 1, mixed flag randomized
- W2S: flag 1, block type 2, mixed flag 0 (short table)
- W2M: flag 1, block type 2, mixed flag 1 (mixed table)
- W3: flag 1, block type 3, mixed flag randomized

Every row randomizes initial bit alignment 0-7, reservoir/scfsi and all decoded
field values, including valid boundaries (`big_values` 0 and 288 and
`scalefac_compress` 499/500 where representable), with a sufficient randomized
bit limit. Table data referenced by `sfbtab` is part of the compared output.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `read_side_info` | P1 + M + N | [x] |
| 2 | `read_side_info` | P1 + M + W1 | [x] |
| 3 | `read_side_info` | P1 + M + W2S | [x] |
| 4 | `read_side_info` | P1 + M + W2M | [x] |
| 5 | `read_side_info` | P1 + M + W3 | [x] |
| 6 | `read_side_info` | P1 + S + N | [x] |
| 7 | `read_side_info` | P1 + S + W1 | [x] |
| 8 | `read_side_info` | P1 + S + W2S | [x] |
| 9 | `read_side_info` | P1 + S + W2M | [x] |
| 10 | `read_side_info` | P1 + S + W3 | [x] |
| 11 | `read_side_info` | P2 + M + N | [x] |
| 12 | `read_side_info` | P2 + M + W1 | [x] |
| 13 | `read_side_info` | P2 + M + W2S | [x] |
| 14 | `read_side_info` | P2 + M + W2M | [x] |
| 15 | `read_side_info` | P2 + M + W3 | [x] |
| 16 | `read_side_info` | P2 + S + N | [x] |
| 17 | `read_side_info` | P2 + S + W1 | [x] |
| 18 | `read_side_info` | P2 + S + W2S | [x] |
| 19 | `read_side_info` | P2 + S + W2M | [x] |
| 20 | `read_side_info` | P2 + S + W3 | [x] |
| 21 | `read_side_info` | P3 + M + N | [x] |
| 22 | `read_side_info` | P3 + M + W1 | [x] |
| 23 | `read_side_info` | P3 + M + W2S | [x] |
| 24 | `read_side_info` | P3 + M + W2M | [x] |
| 25 | `read_side_info` | P3 + M + W3 | [x] |
| 26 | `read_side_info` | P3 + S + N | [x] |
| 27 | `read_side_info` | P3 + S + W1 | [x] |
| 28 | `read_side_info` | P3 + S + W2S | [x] |
| 29 | `read_side_info` | P3 + S + W2M | [x] |
| 30 | `read_side_info` | P3 + S + W3 | [x] |
| 31 | `read_side_info` | P4 + M + N | [x] |
| 32 | `read_side_info` | P4 + M + W1 | [x] |
| 33 | `read_side_info` | P4 + M + W2S | [x] |
| 34 | `read_side_info` | P4 + M + W2M | [x] |
| 35 | `read_side_info` | P4 + M + W3 | [x] |
| 36 | `read_side_info` | P4 + S + N | [x] |
| 37 | `read_side_info` | P4 + S + W1 | [x] |
| 38 | `read_side_info` | P4 + S + W2S | [x] |
| 39 | `read_side_info` | P4 + S + W2M | [x] |
| 40 | `read_side_info` | P4 + S + W3 | [x] |
| 41 | `read_side_info` | P5 + M + N | [x] |
| 42 | `read_side_info` | P5 + M + W1 | [x] |
| 43 | `read_side_info` | P5 + M + W2S | [x] |
| 44 | `read_side_info` | P5 + M + W2M | [x] |
| 45 | `read_side_info` | P5 + M + W3 | [x] |
| 46 | `read_side_info` | P5 + S + N | [x] |
| 47 | `read_side_info` | P5 + S + W1 | [x] |
| 48 | `read_side_info` | P5 + S + W2S | [x] |
| 49 | `read_side_info` | P5 + S + W2M | [x] |
| 50 | `read_side_info` | P5 + S + W3 | [x] |
| 51 | `read_side_info` | P6 + M + N | [x] |
| 52 | `read_side_info` | P6 + M + W1 | [x] |
| 53 | `read_side_info` | P6 + M + W2S | [x] |
| 54 | `read_side_info` | P6 + M + W2M | [x] |
| 55 | `read_side_info` | P6 + M + W3 | [x] |
| 56 | `read_side_info` | P6 + S + N | [x] |
| 57 | `read_side_info` | P6 + S + W1 | [x] |
| 58 | `read_side_info` | P6 + S + W2S | [x] |
| 59 | `read_side_info` | P6 + S + W2M | [x] |
| 60 | `read_side_info` | P6 + S + W3 | [x] |
| 61 | `read_side_info` | P7 + M + N | [x] |
| 62 | `read_side_info` | P7 + M + W1 | [x] |
| 63 | `read_side_info` | P7 + M + W2S | [x] |
| 64 | `read_side_info` | P7 + M + W2M | [x] |
| 65 | `read_side_info` | P7 + M + W3 | [x] |
| 66 | `read_side_info` | P7 + S + N | [x] |
| 67 | `read_side_info` | P7 + S + W1 | [x] |
| 68 | `read_side_info` | P7 + S + W2S | [x] |
| 69 | `read_side_info` | P7 + S + W2M | [x] |
| 70 | `read_side_info` | P7 + S + W3 | [x] |
| 71 | `read_side_info` | P8 + M + N | [x] |
| 72 | `read_side_info` | P8 + M + W1 | [x] |
| 73 | `read_side_info` | P8 + M + W2S | [x] |
| 74 | `read_side_info` | P8 + M + W2M | [x] |
| 75 | `read_side_info` | P8 + M + W3 | [x] |
| 76 | `read_side_info` | P8 + S + N | [x] |
| 77 | `read_side_info` | P8 + S + W1 | [x] |
| 78 | `read_side_info` | P8 + S + W2S | [x] |
| 79 | `read_side_info` | P8 + S + W2M | [x] |
| 80 | `read_side_info` | P8 + S + W3 | [x] |
| 81 | `read_side_info` | P9 + M + N | [x] |
| 82 | `read_side_info` | P9 + M + W1 | [x] |
| 83 | `read_side_info` | P9 + M + W2S | [x] |
| 84 | `read_side_info` | P9 + M + W2M | [x] |
| 85 | `read_side_info` | P9 + M + W3 | [x] |
| 86 | `read_side_info` | P9 + S + N | [x] |
| 87 | `read_side_info` | P9 + S + W1 | [x] |
| 88 | `read_side_info` | P9 + S + W2S | [x] |
| 89 | `read_side_info` | P9 + S + W2M | [x] |
| 90 | `read_side_info` | P9 + S + W3 | [x] |
