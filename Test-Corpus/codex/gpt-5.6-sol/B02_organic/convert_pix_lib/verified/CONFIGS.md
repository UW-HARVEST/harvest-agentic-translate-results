# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
`option`, conditional source, or compile definition. There is exactly one valid
feature combination:

| # | Rust feature combination | matching C configuration | [ ] |
|---|--------------------------|--------------------------|-----|
| 1 | empty set (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

Rows are derived from exported symbols and branch conditions in
`c_src/src/lib.c`. Alignment is included because `cp_inflate` has separate
first-byte, whole-word, and final-byte paths based on both pointer address and
input length.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | exported data symbols | initial bytes and null state of all seven exported globals | [x] |
| 2 | `convert_pix` | `bpp=1`, `w=1`, `h=1` (grayscale, one pixel) | [x] |
| 3 | `convert_pix` | `bpp=1`, `w>1`, `h>1` (grayscale, many rows and columns) | [x] |
| 4 | `convert_pix` | `bpp=2`, `w=1`, `h=1` (grayscale-alpha, one pixel) | [x] |
| 5 | `convert_pix` | `bpp=2`, `w>1`, `h>1` (grayscale-alpha, many rows and columns) | [x] |
| 6 | `convert_pix` | `bpp=3`, `w=1`, `h=1` (RGB, one pixel) | [x] |
| 7 | `convert_pix` | `bpp=3`, `w>1`, `h>1` (RGB, many rows and columns) | [x] |
| 8 | `convert_pix` | `bpp=4`, `w=1`, `h=1` (RGBA, one pixel) | [x] |
| 9 | `convert_pix` | `bpp=4`, `w>1`, `h>1` (RGBA, many rows and columns) | [x] |
| 10 | `convert_pix` | `w=0`, `h>0` (row loop only; one filter byte skipped per row) | [x] |
| 11 | `convert_pix` | `w<0`, `h>0` (same C loop branch as zero width) | [x] |
| 12 | `convert_pix` | `w>0`, `h=0` (no rows) | [x] |
| 13 | `convert_pix` | `w>0`, `h<0` (same C loop branch as zero height) | [x] |
| 14 | `cp_inflate` | final stored block, empty payload, exact output capacity | [x] |
| 15 | `cp_inflate` | final stored block, one-byte payload, exact output capacity | [x] |
| 16 | `cp_inflate` | final stored block, many-byte payload, exact output capacity | [x] |
| 17 | `cp_inflate` | final fixed-Huffman block, empty payload | [x] |
| 18 | `cp_inflate` | final fixed-Huffman block, one literal | [x] |
| 19 | `cp_inflate` | final fixed-Huffman block, many literals and no length/distance pair | [x] |
| 20 | `cp_inflate` | fixed-Huffman length/distance pair with backwards distance `1` | [x] |
| 21 | `cp_inflate` | fixed-Huffman length/distance pair with backwards distance greater than `1` | [x] |
| 22 | `cp_inflate` | final dynamic-Huffman block, empty payload | [x] |
| 23 | `cp_inflate` | final dynamic-Huffman block, one literal | [x] |
| 24 | `cp_inflate` | dynamic-Huffman block using code-length symbol `16` | [x] |
| 25 | `cp_inflate` | dynamic-Huffman block using code-length symbol `17` | [x] |
| 26 | `cp_inflate` | dynamic-Huffman block using code-length symbol `18` | [x] |
| 27 | `cp_inflate` | dynamic-Huffman output containing literals and distance-`1` copies | [x] |
| 28 | `cp_inflate` | dynamic-Huffman output containing a distance greater than `1` | [x] |
| 29 | `cp_inflate` | more than one block with `BFINAL=0` followed by `BFINAL=1` | [x] |
| 30 | `cp_inflate` | valid stream with output capacity exactly equal to output size | [x] |
| 31 | `cp_inflate` | valid stream with output capacity greater than output size | [x] |
| 32 | `cp_inflate` | input pointer address modulo 4 is `0` | [x] |
| 33 | `cp_inflate` | input pointer address modulo 4 is `1` | [x] |
| 34 | `cp_inflate` | input pointer address modulo 4 is `2` | [x] |
| 35 | `cp_inflate` | input pointer address modulo 4 is `3` | [x] |
| 36 | `cp_inflate` | bytes after the initial alignment prefix leave `last_bytes=0` | [x] |
| 37 | `cp_inflate` | bytes after the initial alignment prefix leave `last_bytes=1` | [x] |
| 38 | `cp_inflate` | bytes after the initial alignment prefix leave `last_bytes=2` | [x] |
| 39 | `cp_inflate` | bytes after the initial alignment prefix leave `last_bytes=3` | [x] |
| 40 | `cp_inflate` | randomized incompressible payloads (literal-heavy) | [x] |
| 41 | `cp_inflate` | randomized repetitive payloads (length/distance-heavy) | [x] |

The pointer-alignment rows and `last_bytes` rows form the full `4 x 4`
cross-product in the test loop. Rows 30-31 are crossed with empty, one-byte,
and many-byte output shapes. Each data-bearing row is run with a fixed-seed
random corpus rather than one fixture.
