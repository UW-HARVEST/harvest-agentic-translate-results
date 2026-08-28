# Configuration Surface

The public API has one entry point and no runtime options, modes, flags,
pointers, lengths, enums, or compile-time Cargo features. Its fixed input shape
is two three-byte `cb_rgb_255` values.

The C source independently branches for all six normalized channels at
`0.04045`, then branches on `LumA < LumB`. For byte inputs, `L` means
`0..=10` (the divide-by-12.92 path) and `H` means `11..=255` (the `pow` path).
Masks list `AR AG AB BR BG BB`. The table is the mechanically generated
cross-product of all 64 channel masks and every feasible luminance ordering;
four impossible orderings are pruned by the monotonic endpoint bounds.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 00a | `contrast_ratio` | `LLLLLL`; `LumA < LumB` | [x] |
| 00b | `contrast_ratio` | `LLLLLL`; `LumA >= LumB` | [x] |
| 01a | `contrast_ratio` | `LLLLLH`; `LumA < LumB` | [x] |
| 01b | `contrast_ratio` | `LLLLLH`; `LumA >= LumB` | [x] |
| 02a | `contrast_ratio` | `LLLLHL`; `LumA < LumB` | [x] |
| 02b | `contrast_ratio` | `LLLLHL`; `LumA >= LumB` | [x] |
| 03a | `contrast_ratio` | `LLLLHH`; `LumA < LumB` | [x] |
| 03b | `contrast_ratio` | `LLLLHH`; `LumA >= LumB` | [x] |
| 04a | `contrast_ratio` | `LLLHLL`; `LumA < LumB` | [x] |
| 04b | `contrast_ratio` | `LLLHLL`; `LumA >= LumB` | [x] |
| 05a | `contrast_ratio` | `LLLHLH`; `LumA < LumB` | [x] |
| 05b | `contrast_ratio` | `LLLHLH`; `LumA >= LumB` | [x] |
| 06a | `contrast_ratio` | `LLLHHL`; `LumA < LumB` | [x] |
| 07a | `contrast_ratio` | `LLLHHH`; `LumA < LumB` | [x] |
| 08a | `contrast_ratio` | `LLHLLL`; `LumA < LumB` | [x] |
| 08b | `contrast_ratio` | `LLHLLL`; `LumA >= LumB` | [x] |
| 09a | `contrast_ratio` | `LLHLLH`; `LumA < LumB` | [x] |
| 09b | `contrast_ratio` | `LLHLLH`; `LumA >= LumB` | [x] |
| 10a | `contrast_ratio` | `LLHLHL`; `LumA < LumB` | [x] |
| 10b | `contrast_ratio` | `LLHLHL`; `LumA >= LumB` | [x] |
| 11a | `contrast_ratio` | `LLHLHH`; `LumA < LumB` | [x] |
| 11b | `contrast_ratio` | `LLHLHH`; `LumA >= LumB` | [x] |
| 12a | `contrast_ratio` | `LLHHLL`; `LumA < LumB` | [x] |
| 12b | `contrast_ratio` | `LLHHLL`; `LumA >= LumB` | [x] |
| 13a | `contrast_ratio` | `LLHHLH`; `LumA < LumB` | [x] |
| 13b | `contrast_ratio` | `LLHHLH`; `LumA >= LumB` | [x] |
| 14a | `contrast_ratio` | `LLHHHL`; `LumA < LumB` | [x] |
| 14b | `contrast_ratio` | `LLHHHL`; `LumA >= LumB` | [x] |
| 15a | `contrast_ratio` | `LLHHHH`; `LumA < LumB` | [x] |
| 15b | `contrast_ratio` | `LLHHHH`; `LumA >= LumB` | [x] |
| 16a | `contrast_ratio` | `LHLLLL`; `LumA < LumB` | [x] |
| 16b | `contrast_ratio` | `LHLLLL`; `LumA >= LumB` | [x] |
| 17a | `contrast_ratio` | `LHLLLH`; `LumA < LumB` | [x] |
| 17b | `contrast_ratio` | `LHLLLH`; `LumA >= LumB` | [x] |
| 18a | `contrast_ratio` | `LHLLHL`; `LumA < LumB` | [x] |
| 18b | `contrast_ratio` | `LHLLHL`; `LumA >= LumB` | [x] |
| 19a | `contrast_ratio` | `LHLLHH`; `LumA < LumB` | [x] |
| 19b | `contrast_ratio` | `LHLLHH`; `LumA >= LumB` | [x] |
| 20a | `contrast_ratio` | `LHLHLL`; `LumA < LumB` | [x] |
| 20b | `contrast_ratio` | `LHLHLL`; `LumA >= LumB` | [x] |
| 21a | `contrast_ratio` | `LHLHLH`; `LumA < LumB` | [x] |
| 21b | `contrast_ratio` | `LHLHLH`; `LumA >= LumB` | [x] |
| 22a | `contrast_ratio` | `LHLHHL`; `LumA < LumB` | [x] |
| 22b | `contrast_ratio` | `LHLHHL`; `LumA >= LumB` | [x] |
| 23a | `contrast_ratio` | `LHLHHH`; `LumA < LumB` | [x] |
| 23b | `contrast_ratio` | `LHLHHH`; `LumA >= LumB` | [x] |
| 24a | `contrast_ratio` | `LHHLLL`; `LumA < LumB` | [x] |
| 24b | `contrast_ratio` | `LHHLLL`; `LumA >= LumB` | [x] |
| 25a | `contrast_ratio` | `LHHLLH`; `LumA < LumB` | [x] |
| 25b | `contrast_ratio` | `LHHLLH`; `LumA >= LumB` | [x] |
| 26a | `contrast_ratio` | `LHHLHL`; `LumA < LumB` | [x] |
| 26b | `contrast_ratio` | `LHHLHL`; `LumA >= LumB` | [x] |
| 27a | `contrast_ratio` | `LHHLHH`; `LumA < LumB` | [x] |
| 27b | `contrast_ratio` | `LHHLHH`; `LumA >= LumB` | [x] |
| 28a | `contrast_ratio` | `LHHHLL`; `LumA < LumB` | [x] |
| 28b | `contrast_ratio` | `LHHHLL`; `LumA >= LumB` | [x] |
| 29a | `contrast_ratio` | `LHHHLH`; `LumA < LumB` | [x] |
| 29b | `contrast_ratio` | `LHHHLH`; `LumA >= LumB` | [x] |
| 30a | `contrast_ratio` | `LHHHHL`; `LumA < LumB` | [x] |
| 30b | `contrast_ratio` | `LHHHHL`; `LumA >= LumB` | [x] |
| 31a | `contrast_ratio` | `LHHHHH`; `LumA < LumB` | [x] |
| 31b | `contrast_ratio` | `LHHHHH`; `LumA >= LumB` | [x] |
| 32a | `contrast_ratio` | `HLLLLL`; `LumA < LumB` | [x] |
| 32b | `contrast_ratio` | `HLLLLL`; `LumA >= LumB` | [x] |
| 33a | `contrast_ratio` | `HLLLLH`; `LumA < LumB` | [x] |
| 33b | `contrast_ratio` | `HLLLLH`; `LumA >= LumB` | [x] |
| 34a | `contrast_ratio` | `HLLLHL`; `LumA < LumB` | [x] |
| 34b | `contrast_ratio` | `HLLLHL`; `LumA >= LumB` | [x] |
| 35a | `contrast_ratio` | `HLLLHH`; `LumA < LumB` | [x] |
| 35b | `contrast_ratio` | `HLLLHH`; `LumA >= LumB` | [x] |
| 36a | `contrast_ratio` | `HLLHLL`; `LumA < LumB` | [x] |
| 36b | `contrast_ratio` | `HLLHLL`; `LumA >= LumB` | [x] |
| 37a | `contrast_ratio` | `HLLHLH`; `LumA < LumB` | [x] |
| 37b | `contrast_ratio` | `HLLHLH`; `LumA >= LumB` | [x] |
| 38a | `contrast_ratio` | `HLLHHL`; `LumA < LumB` | [x] |
| 38b | `contrast_ratio` | `HLLHHL`; `LumA >= LumB` | [x] |
| 39a | `contrast_ratio` | `HLLHHH`; `LumA < LumB` | [x] |
| 39b | `contrast_ratio` | `HLLHHH`; `LumA >= LumB` | [x] |
| 40a | `contrast_ratio` | `HLHLLL`; `LumA < LumB` | [x] |
| 40b | `contrast_ratio` | `HLHLLL`; `LumA >= LumB` | [x] |
| 41a | `contrast_ratio` | `HLHLLH`; `LumA < LumB` | [x] |
| 41b | `contrast_ratio` | `HLHLLH`; `LumA >= LumB` | [x] |
| 42a | `contrast_ratio` | `HLHLHL`; `LumA < LumB` | [x] |
| 42b | `contrast_ratio` | `HLHLHL`; `LumA >= LumB` | [x] |
| 43a | `contrast_ratio` | `HLHLHH`; `LumA < LumB` | [x] |
| 43b | `contrast_ratio` | `HLHLHH`; `LumA >= LumB` | [x] |
| 44a | `contrast_ratio` | `HLHHLL`; `LumA < LumB` | [x] |
| 44b | `contrast_ratio` | `HLHHLL`; `LumA >= LumB` | [x] |
| 45a | `contrast_ratio` | `HLHHLH`; `LumA < LumB` | [x] |
| 45b | `contrast_ratio` | `HLHHLH`; `LumA >= LumB` | [x] |
| 46a | `contrast_ratio` | `HLHHHL`; `LumA < LumB` | [x] |
| 46b | `contrast_ratio` | `HLHHHL`; `LumA >= LumB` | [x] |
| 47a | `contrast_ratio` | `HLHHHH`; `LumA < LumB` | [x] |
| 47b | `contrast_ratio` | `HLHHHH`; `LumA >= LumB` | [x] |
| 48b | `contrast_ratio` | `HHLLLL`; `LumA >= LumB` | [x] |
| 49a | `contrast_ratio` | `HHLLLH`; `LumA < LumB` | [x] |
| 49b | `contrast_ratio` | `HHLLLH`; `LumA >= LumB` | [x] |
| 50a | `contrast_ratio` | `HHLLHL`; `LumA < LumB` | [x] |
| 50b | `contrast_ratio` | `HHLLHL`; `LumA >= LumB` | [x] |
| 51a | `contrast_ratio` | `HHLLHH`; `LumA < LumB` | [x] |
| 51b | `contrast_ratio` | `HHLLHH`; `LumA >= LumB` | [x] |
| 52a | `contrast_ratio` | `HHLHLL`; `LumA < LumB` | [x] |
| 52b | `contrast_ratio` | `HHLHLL`; `LumA >= LumB` | [x] |
| 53a | `contrast_ratio` | `HHLHLH`; `LumA < LumB` | [x] |
| 53b | `contrast_ratio` | `HHLHLH`; `LumA >= LumB` | [x] |
| 54a | `contrast_ratio` | `HHLHHL`; `LumA < LumB` | [x] |
| 54b | `contrast_ratio` | `HHLHHL`; `LumA >= LumB` | [x] |
| 55a | `contrast_ratio` | `HHLHHH`; `LumA < LumB` | [x] |
| 55b | `contrast_ratio` | `HHLHHH`; `LumA >= LumB` | [x] |
| 56b | `contrast_ratio` | `HHHLLL`; `LumA >= LumB` | [x] |
| 57a | `contrast_ratio` | `HHHLLH`; `LumA < LumB` | [x] |
| 57b | `contrast_ratio` | `HHHLLH`; `LumA >= LumB` | [x] |
| 58a | `contrast_ratio` | `HHHLHL`; `LumA < LumB` | [x] |
| 58b | `contrast_ratio` | `HHHLHL`; `LumA >= LumB` | [x] |
| 59a | `contrast_ratio` | `HHHLHH`; `LumA < LumB` | [x] |
| 59b | `contrast_ratio` | `HHHLHH`; `LumA >= LumB` | [x] |
| 60a | `contrast_ratio` | `HHHHLL`; `LumA < LumB` | [x] |
| 60b | `contrast_ratio` | `HHHHLL`; `LumA >= LumB` | [x] |
| 61a | `contrast_ratio` | `HHHHLH`; `LumA < LumB` | [x] |
| 61b | `contrast_ratio` | `HHHHLH`; `LumA >= LumB` | [x] |
| 62a | `contrast_ratio` | `HHHHHL`; `LumA < LumB` | [x] |
| 62b | `contrast_ratio` | `HHHHHL`; `LumA >= LumB` | [x] |
| 63a | `contrast_ratio` | `HHHHHH`; `LumA < LumB` | [x] |
| 63b | `contrast_ratio` | `HHHHHH`; `LumA >= LumB` | [x] |

Boundary values `0`, `10`, `11`, and `255`, equal nonzero colors, and the
zero-luminance `0/0` and nonzero-over-zero cases are exercised explicitly in
addition to the randomized cases represented above.
