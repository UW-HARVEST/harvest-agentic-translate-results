# Configuration surface

Rows are derived from the exported functions/data and from branches in
`../c_src/src/lib.c`. There are no Cargo features in `Cargo.toml`, so the only
feature configurations are default and `--no-default-features` (both empty).

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | exported data symbols | all six lookup tables, initial null `cp_error_reason` | [x] |
| 2 | `convert_pix` | `bpp=1`, positive one-row/one-pixel grayscale input | [x] |
| 3 | `convert_pix` | `bpp=1`, positive many-row/many-pixel grayscale input | [x] |
| 4 | `convert_pix` | `bpp=2`, positive grayscale+alpha input | [x] |
| 5 | `convert_pix` | `bpp=3`, positive RGB input | [x] |
| 6 | `convert_pix` | `bpp=4`, positive RGBA input | [x] |
| 7 | `convert_pix` | supported `bpp`, `w=0` with positive `h` | [x] |
| 8 | `convert_pix` | supported `bpp`, positive `w` with `h=0` | [x] |
| 9 | `convert_pix` | supported `bpp`, negative `w` and/or `h` | [x] |
| 10 | `convert_pix` | unsupported `bpp` with zero dimensions (defined no-write path) | [x] |
| 11 | `cp_inflate` | final stored block, empty payload, input pointer alignments 0/1/2/3 mod 4 | [x] |
| 12 | `cp_inflate` | final stored block, nonempty randomized payload, exact output size | [x] |
| 13 | `cp_inflate` | final stored block, nonempty randomized payload, oversized output buffer | [x] |
| 14 | `cp_inflate` | final fixed-Huffman block, literal-only/short randomized data | [x] |
| 15 | `cp_inflate` | final fixed-Huffman block containing distance `1` copies | [x] |
| 16 | `cp_inflate` | final fixed-Huffman block containing distance greater than `1` copies | [x] |
| 17 | `cp_inflate` | final dynamic-Huffman block, randomized compressible data | [x] |
| 18 | `cp_inflate` | final dynamic-Huffman block with distance `1` and distance greater than `1` copies | [x] |
| 19 | `cp_inflate` | multiple non-final/final compressed blocks | [x] |
| 20 | `cp_inflate` | valid fixed/dynamic stream, exact output size | [x] |
| 21 | `cp_inflate` | valid fixed/dynamic stream, oversized output buffer | [x] |
| 22 | `cp_inflate` | valid streams at input pointer alignments 0/1/2/3 mod 4 | [x] |
| 23 | `cp_inflate` + `cp_error_reason` | successful call after a prior error (C leaves the prior reason pointer unchanged) | [x] |
| 24 | all exports | default Cargo feature configuration | [x] |
| 25 | all exports | empty `--no-default-features` configuration | [x] |

The CMake project defines only a shared library; it does not build a binary
driver, so no stdout comparison row applies.
