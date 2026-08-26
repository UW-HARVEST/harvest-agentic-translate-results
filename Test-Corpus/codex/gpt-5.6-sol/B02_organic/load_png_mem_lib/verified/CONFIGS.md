# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake declares no options or
preprocessor definitions. There is exactly one valid build-time combination:

| # | Rust features | C configuration | [x] |
|---|---------------|-----------------|-----|
| B01 | `--no-default-features --features ''` | CMake defaults | [x] |

## Runtime Configurations

Rows are derived from the exported `cp_inflate` and `load_png_mem` call graph.
Filter rows are crossed with bytes-per-pixel because the loops branch on both
filter and `bpp`; randomized cases cover widths at/below/above `bpp`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| D01 | `cp_inflate` | one final stored block; input address aligned mod 4 = 0 | [x] |
| D02 | `cp_inflate` | one final stored block; input address aligned mod 4 = 1 | [x] |
| D03 | `cp_inflate` | one final stored block; input address aligned mod 4 = 2 | [x] |
| D04 | `cp_inflate` | one final stored block; input address aligned mod 4 = 3 | [x] |
| D05 | `cp_inflate` | multiple stored blocks (`BFINAL=0` then `BFINAL=1`) | [x] |
| D06 | `cp_inflate` | final fixed-Huffman block containing only literals | [x] |
| D07 | `cp_inflate` | fixed-Huffman length/distance with distance 1 (`memset` branch) | [x] |
| D08 | `cp_inflate` | fixed-Huffman length/distance with distance greater than 1 (copy loop) | [x] |
| D09 | `cp_inflate` | final dynamic-Huffman block containing literals | [x] |
| D10 | `cp_inflate` | dynamic-Huffman length/distance data | [x] |
| D11 | `cp_inflate` | output length exactly produced length | [x] |
| D12 | `cp_inflate` | output capacity larger than produced length | [x] |
| P01 | `load_png_mem` | color type 0, grayscale (`bpp=1`), height 1, filter 0 | [x] |
| P02 | `load_png_mem` | color type 2, RGB (`bpp=3`), height 1, filter 0 | [x] |
| P03 | `load_png_mem` | color type 3, indexed (`bpp=1`), PLTE, no tRNS | [x] |
| P04 | `load_png_mem` | color type 3, PLTE and partial tRNS; indices both below and at/above `trns_len` | [x] |
| P05 | `load_png_mem` | color type 3, PLTE and full tRNS | [x] |
| P06 | `load_png_mem` | color type 4, grayscale+alpha (`bpp=2`), height 1, filter 0 | [x] |
| P07 | `load_png_mem` | color type 6, RGBA (`bpp=4`), height 1, filter 0 | [x] |
| P08 | `load_png_mem` | IHDR encoded width 0, which C exposes as image width 0 | [x] |
| P09 | `load_png_mem` | width 1 | [x] |
| P10 | `load_png_mem` | width greater than `bpp`, randomized width and height | [x] |
| P11 | `load_png_mem` | one IDAT chunk | [x] |
| P12 | `load_png_mem` | compressed stream split across adjacent IDAT chunks | [x] |
| P13 | `load_png_mem` | ancillary chunk before IDAT; `cp_find` skips it | [x] |
| P14 | `load_png_mem` | PNG zlib payload using stored DEFLATE blocks | [x] |
| P15 | `load_png_mem` | PNG zlib payload using fixed-Huffman DEFLATE | [x] |
| P16 | `load_png_mem` | PNG zlib payload using dynamic-Huffman DEFLATE | [x] |
| F01 | `load_png_mem` | first row filter 0 crossed with `bpp=1,2,3,4` | [x] |
| F02 | `load_png_mem` | first row filter 1 crossed with `bpp=1,2,3,4`; widths below/equal/above bpp | [x] |
| F03 | `load_png_mem` | first row filter 2 crossed with `bpp=1,2,3,4` (C leaves row unchanged) | [x] |
| F04 | `load_png_mem` | first row filter 3 crossed with `bpp=1,2,3,4`; leading bytes unchanged | [x] |
| F05 | `load_png_mem` | first row filter 4 crossed with `bpp=1,2,3,4`; leading bytes unchanged | [x] |
| F06 | `load_png_mem` | later row filter 0 crossed with `bpp=1,2,3,4` | [x] |
| F07 | `load_png_mem` | later row filter 1 crossed with `bpp=1,2,3,4` | [x] |
| F08 | `load_png_mem` | later row filter 2 crossed with `bpp=1,2,3,4` | [x] |
| F09 | `load_png_mem` | later row filter 3 crossed with `bpp=1,2,3,4` | [x] |
| F10 | `load_png_mem` | later row filter 4 crossed with `bpp=1,2,3,4` | [x] |
