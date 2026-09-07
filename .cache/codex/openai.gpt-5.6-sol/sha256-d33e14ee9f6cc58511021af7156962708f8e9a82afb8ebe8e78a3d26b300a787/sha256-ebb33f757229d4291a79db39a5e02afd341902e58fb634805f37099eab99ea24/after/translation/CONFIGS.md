# Configuration-surface table

The public surface contains one entry point, no runtime options, and one fixed
input shape: two by-value RGB structs containing six `unsigned char` channels.

`cbLuminance` branches independently for every channel at normalized value
`0.04045`. For the public byte API this partitions each channel into:

- `L`: byte values `0..=10`, taking the linear `channel / 12.92` branch;
- `N`: byte values `11..=255`, taking the nonlinear `pow(...)` branch.

The rows below mechanically enumerate the full `2^6` branch cross-product in
the order `A.R A.G A.B B.R B.G B.B`. Each row is exercised with fixed-seed
random inputs from every listed range.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `contrast_ratio` | `LLLLLL` | [x] |
| 2 | `contrast_ratio` | `LLLLLN` | [x] |
| 3 | `contrast_ratio` | `LLLLNL` | [x] |
| 4 | `contrast_ratio` | `LLLLNN` | [x] |
| 5 | `contrast_ratio` | `LLLNLL` | [x] |
| 6 | `contrast_ratio` | `LLLNLN` | [x] |
| 7 | `contrast_ratio` | `LLLNNL` | [x] |
| 8 | `contrast_ratio` | `LLLNNN` | [x] |
| 9 | `contrast_ratio` | `LLNLLL` | [x] |
| 10 | `contrast_ratio` | `LLNLLN` | [x] |
| 11 | `contrast_ratio` | `LLNLNL` | [x] |
| 12 | `contrast_ratio` | `LLNLNN` | [x] |
| 13 | `contrast_ratio` | `LLNNLL` | [x] |
| 14 | `contrast_ratio` | `LLNNLN` | [x] |
| 15 | `contrast_ratio` | `LLNNNL` | [x] |
| 16 | `contrast_ratio` | `LLNNNN` | [x] |
| 17 | `contrast_ratio` | `LNLLLL` | [x] |
| 18 | `contrast_ratio` | `LNLLLN` | [x] |
| 19 | `contrast_ratio` | `LNLLNL` | [x] |
| 20 | `contrast_ratio` | `LNLLNN` | [x] |
| 21 | `contrast_ratio` | `LNLNLL` | [x] |
| 22 | `contrast_ratio` | `LNLNLN` | [x] |
| 23 | `contrast_ratio` | `LNLNNL` | [x] |
| 24 | `contrast_ratio` | `LNLNNN` | [x] |
| 25 | `contrast_ratio` | `LNNLLL` | [x] |
| 26 | `contrast_ratio` | `LNNLLN` | [x] |
| 27 | `contrast_ratio` | `LNNLNL` | [x] |
| 28 | `contrast_ratio` | `LNNLNN` | [x] |
| 29 | `contrast_ratio` | `LNNNLL` | [x] |
| 30 | `contrast_ratio` | `LNNNLN` | [x] |
| 31 | `contrast_ratio` | `LNNNNL` | [x] |
| 32 | `contrast_ratio` | `LNNNNN` | [x] |
| 33 | `contrast_ratio` | `NLLLLL` | [x] |
| 34 | `contrast_ratio` | `NLLLLN` | [x] |
| 35 | `contrast_ratio` | `NLLLNL` | [x] |
| 36 | `contrast_ratio` | `NLLLNN` | [x] |
| 37 | `contrast_ratio` | `NLLNLL` | [x] |
| 38 | `contrast_ratio` | `NLLNLN` | [x] |
| 39 | `contrast_ratio` | `NLLNNL` | [x] |
| 40 | `contrast_ratio` | `NLLNNN` | [x] |
| 41 | `contrast_ratio` | `NLNLLL` | [x] |
| 42 | `contrast_ratio` | `NLNLLN` | [x] |
| 43 | `contrast_ratio` | `NLNLNL` | [x] |
| 44 | `contrast_ratio` | `NLNLNN` | [x] |
| 45 | `contrast_ratio` | `NLNNLL` | [x] |
| 46 | `contrast_ratio` | `NLNNLN` | [x] |
| 47 | `contrast_ratio` | `NLNNNL` | [x] |
| 48 | `contrast_ratio` | `NLNNNN` | [x] |
| 49 | `contrast_ratio` | `NNLLLL` | [x] |
| 50 | `contrast_ratio` | `NNLLLN` | [x] |
| 51 | `contrast_ratio` | `NNLLNL` | [x] |
| 52 | `contrast_ratio` | `NNLLNN` | [x] |
| 53 | `contrast_ratio` | `NNLNLL` | [x] |
| 54 | `contrast_ratio` | `NNLNLN` | [x] |
| 55 | `contrast_ratio` | `NNLNNL` | [x] |
| 56 | `contrast_ratio` | `NNLNNN` | [x] |
| 57 | `contrast_ratio` | `NNNLLL` | [x] |
| 58 | `contrast_ratio` | `NNNLLN` | [x] |
| 59 | `contrast_ratio` | `NNNLNL` | [x] |
| 60 | `contrast_ratio` | `NNNLNN` | [x] |
| 61 | `contrast_ratio` | `NNNNLL` | [x] |
| 62 | `contrast_ratio` | `NNNNLN` | [x] |
| 63 | `contrast_ratio` | `NNNNNL` | [x] |
| 64 | `contrast_ratio` | `NNNNNN` | [x] |

The ratio calculation has one ordering branch and IEEE-754 singular outcomes.
These rows supplement the channel cross-product with exact branch boundaries
and result shapes:

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 65 | `contrast_ratio` | `LumA < LumB` (swap branch taken) | [x] |
| 66 | `contrast_ratio` | `LumA >= LumB` (swap branch not taken, including equal colors) | [x] |
| 67 | `contrast_ratio` | every channel `0` (`0.0 / 0.0`, NaN result) | [x] |
| 68 | `contrast_ratio` | exactly one black color (positive luminance divided by zero, +infinity) | [x] |
| 69 | `contrast_ratio` | threshold boundary byte `10` in every channel | [x] |
| 70 | `contrast_ratio` | first nonlinear byte `11` in every channel | [x] |
| 71 | `contrast_ratio` | byte extrema `0` and `255` across both operands | [x] |

There are no Cargo features and no C preprocessor feature branches, so the
default/no-default build is the complete feature-combination surface.
