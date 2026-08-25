# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake declares no options or
conditional sources. There is one valid build-time configuration:

| # | Cargo invocation | CMake configuration | [ ] |
|---|---|---|---|
| B1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime Configurations

There are no runtime options, modes, flags, pointers, lengths, enums, or
alternate entry points. The sole public entry point is `contrast_ratio`.

For rows T00-T63, the six-bit threshold mask is ordered as
`A.R A.G A.B B.R B.G B.B`. `0` means the C linear branch
`channel / 12.92` (`channel` in `0..=10` after the public API's `/ 255.f`);
`1` means the C power branch (`channel` in `11..=255`). Each row exercises
many fixed-seed randomized values and both argument orders, covering the
`LumA < LumB` swap branch wherever the shape permits it.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|---|---|---|
| T00 | `contrast_ratio` | threshold mask `000000`; randomized channels | [x] |
| T01 | `contrast_ratio` | threshold mask `000001`; randomized channels | [x] |
| T02 | `contrast_ratio` | threshold mask `000010`; randomized channels | [x] |
| T03 | `contrast_ratio` | threshold mask `000011`; randomized channels | [x] |
| T04 | `contrast_ratio` | threshold mask `000100`; randomized channels | [x] |
| T05 | `contrast_ratio` | threshold mask `000101`; randomized channels | [x] |
| T06 | `contrast_ratio` | threshold mask `000110`; randomized channels | [x] |
| T07 | `contrast_ratio` | threshold mask `000111`; randomized channels | [x] |
| T08 | `contrast_ratio` | threshold mask `001000`; randomized channels | [x] |
| T09 | `contrast_ratio` | threshold mask `001001`; randomized channels | [x] |
| T10 | `contrast_ratio` | threshold mask `001010`; randomized channels | [x] |
| T11 | `contrast_ratio` | threshold mask `001011`; randomized channels | [x] |
| T12 | `contrast_ratio` | threshold mask `001100`; randomized channels | [x] |
| T13 | `contrast_ratio` | threshold mask `001101`; randomized channels | [x] |
| T14 | `contrast_ratio` | threshold mask `001110`; randomized channels | [x] |
| T15 | `contrast_ratio` | threshold mask `001111`; randomized channels | [x] |
| T16 | `contrast_ratio` | threshold mask `010000`; randomized channels | [x] |
| T17 | `contrast_ratio` | threshold mask `010001`; randomized channels | [x] |
| T18 | `contrast_ratio` | threshold mask `010010`; randomized channels | [x] |
| T19 | `contrast_ratio` | threshold mask `010011`; randomized channels | [x] |
| T20 | `contrast_ratio` | threshold mask `010100`; randomized channels | [x] |
| T21 | `contrast_ratio` | threshold mask `010101`; randomized channels | [x] |
| T22 | `contrast_ratio` | threshold mask `010110`; randomized channels | [x] |
| T23 | `contrast_ratio` | threshold mask `010111`; randomized channels | [x] |
| T24 | `contrast_ratio` | threshold mask `011000`; randomized channels | [x] |
| T25 | `contrast_ratio` | threshold mask `011001`; randomized channels | [x] |
| T26 | `contrast_ratio` | threshold mask `011010`; randomized channels | [x] |
| T27 | `contrast_ratio` | threshold mask `011011`; randomized channels | [x] |
| T28 | `contrast_ratio` | threshold mask `011100`; randomized channels | [x] |
| T29 | `contrast_ratio` | threshold mask `011101`; randomized channels | [x] |
| T30 | `contrast_ratio` | threshold mask `011110`; randomized channels | [x] |
| T31 | `contrast_ratio` | threshold mask `011111`; randomized channels | [x] |
| T32 | `contrast_ratio` | threshold mask `100000`; randomized channels | [x] |
| T33 | `contrast_ratio` | threshold mask `100001`; randomized channels | [x] |
| T34 | `contrast_ratio` | threshold mask `100010`; randomized channels | [x] |
| T35 | `contrast_ratio` | threshold mask `100011`; randomized channels | [x] |
| T36 | `contrast_ratio` | threshold mask `100100`; randomized channels | [x] |
| T37 | `contrast_ratio` | threshold mask `100101`; randomized channels | [x] |
| T38 | `contrast_ratio` | threshold mask `100110`; randomized channels | [x] |
| T39 | `contrast_ratio` | threshold mask `100111`; randomized channels | [x] |
| T40 | `contrast_ratio` | threshold mask `101000`; randomized channels | [x] |
| T41 | `contrast_ratio` | threshold mask `101001`; randomized channels | [x] |
| T42 | `contrast_ratio` | threshold mask `101010`; randomized channels | [x] |
| T43 | `contrast_ratio` | threshold mask `101011`; randomized channels | [x] |
| T44 | `contrast_ratio` | threshold mask `101100`; randomized channels | [x] |
| T45 | `contrast_ratio` | threshold mask `101101`; randomized channels | [x] |
| T46 | `contrast_ratio` | threshold mask `101110`; randomized channels | [x] |
| T47 | `contrast_ratio` | threshold mask `101111`; randomized channels | [x] |
| T48 | `contrast_ratio` | threshold mask `110000`; randomized channels | [x] |
| T49 | `contrast_ratio` | threshold mask `110001`; randomized channels | [x] |
| T50 | `contrast_ratio` | threshold mask `110010`; randomized channels | [x] |
| T51 | `contrast_ratio` | threshold mask `110011`; randomized channels | [x] |
| T52 | `contrast_ratio` | threshold mask `110100`; randomized channels | [x] |
| T53 | `contrast_ratio` | threshold mask `110101`; randomized channels | [x] |
| T54 | `contrast_ratio` | threshold mask `110110`; randomized channels | [x] |
| T55 | `contrast_ratio` | threshold mask `110111`; randomized channels | [x] |
| T56 | `contrast_ratio` | threshold mask `111000`; randomized channels | [x] |
| T57 | `contrast_ratio` | threshold mask `111001`; randomized channels | [x] |
| T58 | `contrast_ratio` | threshold mask `111010`; randomized channels | [x] |
| T59 | `contrast_ratio` | threshold mask `111011`; randomized channels | [x] |
| T60 | `contrast_ratio` | threshold mask `111100`; randomized channels | [x] |
| T61 | `contrast_ratio` | threshold mask `111101`; randomized channels | [x] |
| T62 | `contrast_ratio` | threshold mask `111110`; randomized channels | [x] |
| T63 | `contrast_ratio` | threshold mask `111111`; randomized channels | [x] |
| S1 | `contrast_ratio` | `A = B = (0,0,0)`; `0.0 / 0.0` produces NaN | [x] |
| S2 | `contrast_ratio` | `A = (0,0,0)`, nonblack `B`; swap then divide by zero | [x] |
| S3 | `contrast_ratio` | nonblack `A`, `B = (0,0,0)`; no swap then divide by zero | [x] |
| S4 | `contrast_ratio` | equal nonblack colors; equality takes no-swap branch | [x] |
| S5 | `contrast_ratio` | threshold-adjacent channel values `10` and `11` | [x] |
| S6 | `contrast_ratio` | channel-domain endpoints `0` and `255` | [x] |
