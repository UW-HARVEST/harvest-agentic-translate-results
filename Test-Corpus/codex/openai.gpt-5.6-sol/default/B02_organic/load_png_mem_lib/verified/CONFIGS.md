# Configuration-surface table

The public dynamic surface has two callable entry points (`cp_inflate` and
`load_png_mem`) plus seven mutable data symbols. The rows below are the
cross-product branches selected by those APIs. Each randomized PNG row covers
`1x1`, `1xN`, `Nx1`, and `NxM` dimensions and exercises its filter on both the
first scanline and, where height permits, later scanlines.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | exported data symbols | exact byte contents and lengths of all six DEFLATE tables plus initial null `cp_error_reason` | [x] |
| 2 | `cp_inflate` | one final stored block, empty payload, input address alignment offsets `0..=3` | [x] |
| 3 | `cp_inflate` | one final stored block, randomized nonempty literal bytes, input alignment offsets `0..=3`, exact and oversized output buffers | [x] |
| 4 | `cp_inflate` | one final fixed-Huffman block containing randomized literals only | [x] |
| 5 | `cp_inflate` | fixed-Huffman block using length/distance with `backwards_distance == 1` (`memset` branch) | [x] |
| 6 | `cp_inflate` | fixed-Huffman block using length/distance `> 1`, including overlapping copies | [x] |
| 7 | `cp_inflate` | dynamic-Huffman block with literal/end symbols and explicit code lengths | [x] |
| 8 | `cp_inflate` | dynamic code-length alphabet exercises direct lengths and repeat symbols `16`, `17`, and `18` | [x] |
| 9 | `cp_inflate` | multiple blocks with `BFINAL=0` followed by `BFINAL=1`, across stored/fixed/dynamic combinations accepted by C | [x] |
| 10 | `cp_inflate` | decoded output length exactly zero, one, and many bytes; output capacity exact and larger | [x] |
| 11 | `load_png_mem` | color type `0` (grayscale, bpp 1), scanline filter `0`, randomized dimensions/pixels | [x] |
| 12 | `load_png_mem` | color type `0` (grayscale, bpp 1), scanline filter `1`, randomized dimensions/pixels | [x] |
| 13 | `load_png_mem` | color type `0` (grayscale, bpp 1), scanline filter `2`, randomized dimensions/pixels | [x] |
| 14 | `load_png_mem` | color type `0` (grayscale, bpp 1), scanline filter `3`, randomized dimensions/pixels | [x] |
| 15 | `load_png_mem` | color type `0` (grayscale, bpp 1), scanline filter `4`, randomized dimensions/pixels | [x] |
| 16 | `load_png_mem` | color type `2` (RGB, bpp 3), scanline filter `0`, randomized dimensions/pixels | [x] |
| 17 | `load_png_mem` | color type `2` (RGB, bpp 3), scanline filter `1`, randomized dimensions/pixels | [x] |
| 18 | `load_png_mem` | color type `2` (RGB, bpp 3), scanline filter `2`, randomized dimensions/pixels | [x] |
| 19 | `load_png_mem` | color type `2` (RGB, bpp 3), scanline filter `3`, randomized dimensions/pixels | [x] |
| 20 | `load_png_mem` | color type `2` (RGB, bpp 3), scanline filter `4`, randomized dimensions/pixels | [x] |
| 21 | `load_png_mem` | color type `3` (indexed, bpp 1), scanline filter `0`, PLTE present | [x] |
| 22 | `load_png_mem` | color type `3` (indexed, bpp 1), scanline filter `1`, PLTE present | [x] |
| 23 | `load_png_mem` | color type `3` (indexed, bpp 1), scanline filter `2`, PLTE present | [x] |
| 24 | `load_png_mem` | color type `3` (indexed, bpp 1), scanline filter `3`, PLTE present | [x] |
| 25 | `load_png_mem` | color type `3` (indexed, bpp 1), scanline filter `4`, PLTE present | [x] |
| 26 | `load_png_mem` | color type `4` (grayscale+alpha, bpp 2), scanline filter `0`, randomized dimensions/pixels | [x] |
| 27 | `load_png_mem` | color type `4` (grayscale+alpha, bpp 2), scanline filter `1`, randomized dimensions/pixels | [x] |
| 28 | `load_png_mem` | color type `4` (grayscale+alpha, bpp 2), scanline filter `2`, randomized dimensions/pixels | [x] |
| 29 | `load_png_mem` | color type `4` (grayscale+alpha, bpp 2), scanline filter `3`, randomized dimensions/pixels | [x] |
| 30 | `load_png_mem` | color type `4` (grayscale+alpha, bpp 2), scanline filter `4`, randomized dimensions/pixels | [x] |
| 31 | `load_png_mem` | color type `6` (RGBA, bpp 4), scanline filter `0`, randomized dimensions/pixels | [x] |
| 32 | `load_png_mem` | color type `6` (RGBA, bpp 4), scanline filter `1`, randomized dimensions/pixels | [x] |
| 33 | `load_png_mem` | color type `6` (RGBA, bpp 4), scanline filter `2`, randomized dimensions/pixels | [x] |
| 34 | `load_png_mem` | color type `6` (RGBA, bpp 4), scanline filter `3`, randomized dimensions/pixels | [x] |
| 35 | `load_png_mem` | color type `6` (RGBA, bpp 4), scanline filter `4`, randomized dimensions/pixels | [x] |
| 36 | `load_png_mem` | indexed PLTE with no tRNS: every palette index gets alpha 255 | [x] |
| 37 | `load_png_mem` | indexed PLTE with tRNS shorter than the used index range: in-range alpha from tRNS, later indices alpha 255 | [x] |
| 38 | `load_png_mem` | indexed PLTE with tRNS covering all used indices | [x] |
| 39 | `load_png_mem` | one IDAT chunk containing the complete zlib stream | [x] |
| 40 | `load_png_mem` | zlib stream split across two or more consecutive IDAT chunks, including splits at header/DEFLATE/trailer boundaries | [x] |
| 41 | `load_png_mem` | ancillary chunks before PLTE/tRNS/IDAT and between searchable chunk groups | [x] |
| 42 | `load_png_mem` | PLTE present for a non-indexed color type (found but conversion remains non-indexed) | [x] |
| 43 | `load_png_mem` | tRNS absent/present for a non-indexed color type (found but ignored by conversion) | [x] |

Cargo features declared in `Cargo.toml`: **none**. Therefore there is one code
configuration; both the normal invocation and `--no-default-features` must pass.
