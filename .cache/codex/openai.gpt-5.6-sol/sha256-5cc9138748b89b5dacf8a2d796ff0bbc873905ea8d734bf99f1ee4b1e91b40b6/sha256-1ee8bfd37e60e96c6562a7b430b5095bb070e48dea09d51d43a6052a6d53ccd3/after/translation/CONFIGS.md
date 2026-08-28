# Configuration surface

`Cargo.toml` declares no features. The only build configuration is the default
configuration (equivalent to `--no-default-features`).

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| 1 | `convert_pix` | `bpp=1`, positive one/many width and height; grayscale becomes opaque RGBA | [x] |
| 2 | `convert_pix` | `bpp=2`, positive one/many width and height; grayscale plus alpha | [x] |
| 3 | `convert_pix` | `bpp=3`, positive one/many width and height; RGB becomes opaque RGBA | [x] |
| 4 | `convert_pix` | `bpp=4`, positive one/many width and height; RGBA preserved | [x] |
| 5 | `convert_pix` | `h=0` or `h<0`; no source or destination access, including null pointers | [x] |
| 6 | `convert_pix` | positive `h` with `w=0` or `w<0`; one filter-byte step per row and no destination writes | [x] |
| 7 | `convert_pix` | unsupported `bpp` (`0` or `>4`) with positive dimensions; destination advances but remains unchanged | [x] |
| 8 | `cp_inflate` | final stored block (`BTYPE=0`), empty and random nonempty payloads, exact input extent | [x] |
| 9 | `cp_inflate` | final fixed-Huffman block (`BTYPE=1`) containing random literals only | [x] |
| 10 | `cp_inflate` | fixed-Huffman length/distance copy with `backwards_distance=1` (`memset` branch) | [x] |
| 11 | `cp_inflate` | fixed-Huffman length/distance copy with `backwards_distance>1` (byte-copy branch) | [x] |
| 12 | `cp_inflate` | final dynamic-Huffman block (`BTYPE=2`) with literal and end symbols | [x] |
| 13 | `cp_inflate` | dynamic-Huffman block containing length/distance symbols and repeated code-length encodings (`16`, `17`, or `18`) | [x] |
| 14 | `cp_inflate` | multiple blocks (`BFINAL=0` followed by `BFINAL=1`) | [x] |
| 15 | `cp_inflate` | input pointer alignment offsets `0`, `1`, `2`, and `3` modulo four | [x] |
| 16 | `cp_inflate` | input tail sizes `0`, `1`, `2`, and `3` modulo four | [x] |
| 17 | `cp_inflate` | exact-size and oversized output buffers for valid streams | [x] |
| 18 | exported data | all six DEFLATE tables match byte-for-byte; `cp_error_reason` is null before any error | [x] |

The DEFLATE rows are the pruned branch cross-product from `BTYPE`, `BFINAL`,
literal/length/end symbol handling, distance `1` versus other distances,
input alignment, tail handling, and output capacity. Randomized cases vary
payload values and empty/one/many lengths within every applicable row.
