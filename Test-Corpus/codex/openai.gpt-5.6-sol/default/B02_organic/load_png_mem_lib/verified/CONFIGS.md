# Configuration Surface

Derived from both exported entry points and every `if`/`switch` affecting a
valid result in `../c_src/src/lib.c`. Rows C18-C67 expand the actual
`color_type`/`bpp`, first-vs-later row, and filter-byte branches. Each row is
run with fixed-seed randomized dimensions and channel bytes.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| C01 | `cp_inflate` | stored (`BTYPE=0`), final, empty payload, exact zero output | [x] |
| C02 | `cp_inflate` | stored, final, randomized nonempty payload, exact output | [x] |
| C03 | `cp_inflate` | stored, randomized payload lengths including byte/word boundaries | [x] |
| C04 | `cp_inflate` | fixed Huffman (`BTYPE=1`), literals and end marker | [x] |
| C05 | `cp_inflate` | fixed Huffman match with backwards distance `1` (`memset` branch) | [x] |
| C06 | `cp_inflate` | fixed Huffman match with backwards distance `>1` (copy loop) | [x] |
| C07 | `cp_inflate` | dynamic Huffman (`BTYPE=2`) with literal values | [x] |
| C08 | `cp_inflate` | dynamic tree using code-length repeat symbol `16` | [x] |
| C09 | `cp_inflate` | dynamic tree using short zero-run symbol `17` | [x] |
| C10 | `cp_inflate` | dynamic tree using long zero-run symbol `18` | [x] |
| C11 | `cp_inflate` | dynamic data with distance `1` match | [x] |
| C12 | `cp_inflate` | dynamic data with distance `>1` match | [x] |
| C13 | `cp_inflate` | mixed/multiple blocks, final flag false then true | [x] |
| C14 | `cp_inflate` | input pointer alignments modulo 4 equal to `0`, `1`, `2`, and `3` | [x] |
| C15 | `cp_inflate` | exact-sized output buffer | [x] |
| C16 | `cp_inflate` | oversized output buffer with unchanged suffix | [x] |
| C17 | `cp_inflate` | empty logical content ending without a literal or match | [x] |
| C18 | `load_png_mem` | color 0, bpp 1, first/only row, filter 0 | [x] |
| C19 | `load_png_mem` | color 0, bpp 1, first/only row, filter 1 | [x] |
| C20 | `load_png_mem` | color 0, bpp 1, first/only row, filter 2 | [x] |
| C21 | `load_png_mem` | color 0, bpp 1, first/only row, filter 3 | [x] |
| C22 | `load_png_mem` | color 0, bpp 1, first/only row, filter 4 | [x] |
| C23 | `load_png_mem` | color 0, bpp 1, later row, filter 0 | [x] |
| C24 | `load_png_mem` | color 0, bpp 1, later row, filter 1 | [x] |
| C25 | `load_png_mem` | color 0, bpp 1, later row, filter 2 | [x] |
| C26 | `load_png_mem` | color 0, bpp 1, later row, filter 3 | [x] |
| C27 | `load_png_mem` | color 0, bpp 1, later row, filter 4 | [x] |
| C28 | `load_png_mem` | color 2, bpp 3, first/only row, filter 0 | [x] |
| C29 | `load_png_mem` | color 2, bpp 3, first/only row, filter 1 | [x] |
| C30 | `load_png_mem` | color 2, bpp 3, first/only row, filter 2 | [x] |
| C31 | `load_png_mem` | color 2, bpp 3, first/only row, filter 3 | [x] |
| C32 | `load_png_mem` | color 2, bpp 3, first/only row, filter 4 | [x] |
| C33 | `load_png_mem` | color 2, bpp 3, later row, filter 0 | [x] |
| C34 | `load_png_mem` | color 2, bpp 3, later row, filter 1 | [x] |
| C35 | `load_png_mem` | color 2, bpp 3, later row, filter 2 | [x] |
| C36 | `load_png_mem` | color 2, bpp 3, later row, filter 3 | [x] |
| C37 | `load_png_mem` | color 2, bpp 3, later row, filter 4 | [x] |
| C38 | `load_png_mem` | color 3, bpp 1, first/only row, filter 0 | [x] |
| C39 | `load_png_mem` | color 3, bpp 1, first/only row, filter 1 | [x] |
| C40 | `load_png_mem` | color 3, bpp 1, first/only row, filter 2 | [x] |
| C41 | `load_png_mem` | color 3, bpp 1, first/only row, filter 3 | [x] |
| C42 | `load_png_mem` | color 3, bpp 1, first/only row, filter 4 | [x] |
| C43 | `load_png_mem` | color 3, bpp 1, later row, filter 0 | [x] |
| C44 | `load_png_mem` | color 3, bpp 1, later row, filter 1 | [x] |
| C45 | `load_png_mem` | color 3, bpp 1, later row, filter 2 | [x] |
| C46 | `load_png_mem` | color 3, bpp 1, later row, filter 3 | [x] |
| C47 | `load_png_mem` | color 3, bpp 1, later row, filter 4 | [x] |
| C48 | `load_png_mem` | color 4, bpp 2, first/only row, filter 0 | [x] |
| C49 | `load_png_mem` | color 4, bpp 2, first/only row, filter 1 | [x] |
| C50 | `load_png_mem` | color 4, bpp 2, first/only row, filter 2 | [x] |
| C51 | `load_png_mem` | color 4, bpp 2, first/only row, filter 3 | [x] |
| C52 | `load_png_mem` | color 4, bpp 2, first/only row, filter 4 | [x] |
| C53 | `load_png_mem` | color 4, bpp 2, later row, filter 0 | [x] |
| C54 | `load_png_mem` | color 4, bpp 2, later row, filter 1 | [x] |
| C55 | `load_png_mem` | color 4, bpp 2, later row, filter 2 | [x] |
| C56 | `load_png_mem` | color 4, bpp 2, later row, filter 3 | [x] |
| C57 | `load_png_mem` | color 4, bpp 2, later row, filter 4 | [x] |
| C58 | `load_png_mem` | color 6, bpp 4, first/only row, filter 0 | [x] |
| C59 | `load_png_mem` | color 6, bpp 4, first/only row, filter 1 | [x] |
| C60 | `load_png_mem` | color 6, bpp 4, first/only row, filter 2 | [x] |
| C61 | `load_png_mem` | color 6, bpp 4, first/only row, filter 3 | [x] |
| C62 | `load_png_mem` | color 6, bpp 4, first/only row, filter 4 | [x] |
| C63 | `load_png_mem` | color 6, bpp 4, later row, filter 0 | [x] |
| C64 | `load_png_mem` | color 6, bpp 4, later row, filter 1 | [x] |
| C65 | `load_png_mem` | color 6, bpp 4, later row, filter 2 | [x] |
| C66 | `load_png_mem` | color 6, bpp 4, later row, filter 3 | [x] |
| C67 | `load_png_mem` | color 6, bpp 4, later row, filter 4 | [x] |
| C68 | `load_png_mem` | indexed palette with no `tRNS` (alpha defaults to 255) | [x] |
| C69 | `load_png_mem` | indexed palette with partial `tRNS`; index both below and at/above `trns_len` | [x] |
| C70 | `load_png_mem` | indexed palette with transparency for every used index | [x] |
| C71 | `load_png_mem` | dimensions `1x1`, `1xN`, `Nx1`, and `NxM` | [x] |
| C72 | `load_png_mem` | one IDAT chunk | [x] |
| C73 | `load_png_mem` | multiple consecutive IDAT chunks, including uneven/empty splits | [x] |
| C74 | `load_png_mem` | unknown ancillary chunks before PLTE, tRNS, and IDAT search targets | [x] |
| C75 | `load_png_mem` | PLTE absent for a non-indexed image (search pointer reset) | [x] |
| C76 | `load_png_mem` | tRNS absent (search pointer reset) | [x] |
| C77 | both | randomized byte values including `0`, `1`, `127`, `128`, `254`, and `255` | [x] |

No Cargo features are declared, so the feature-combination set contains exactly
the default/no-feature build.

All rows pass under both `cargo test` and `cargo test --no-default-features`.
Randomized cases use fixed seeds, and every call to either implementation is
resolved from its shared library with `libloading`.
