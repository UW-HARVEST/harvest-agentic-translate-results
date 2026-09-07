# Configuration surface

Derived from the public header, all C dynamic exports, and every branch in
`cp_inflate`, its reachable helpers, and `unfilter`. “All alignments” means
input addresses modulo four `0, 1, 2, 3`. Randomized data is used for every
applicable row.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | data export | `cp_fixed_table`, all 320 bytes | [x] |
| 2 | data export | `cp_permutation_order`, all 19 bytes | [x] |
| 3 | data export | `cp_len_extra_bits`, all 31 bytes | [x] |
| 4 | data export | `cp_len_base`, all 31 words | [x] |
| 5 | data export | `cp_dist_extra_bits`, all 32 bytes | [x] |
| 6 | data export | `cp_dist_base`, all 32 words | [x] |
| 7 | data export | initial `cp_error_reason == NULL` in a newly loaded object | [x] |
| 8 | `unfilter` | `h <= 0`; null and non-null raw pointers | [x] |
| 9 | `unfilter` | first row filter 0; zero-length row | [x] |
| 10 | `unfilter` | first row filter 0; one pixel (`len == bpp`) | [x] |
| 11 | `unfilter` | first row filter 0; multiple pixels (`len > bpp`) | [x] |
| 12 | `unfilter` | first row filter 1; zero-length row | [x] |
| 13 | `unfilter` | first row filter 1; one pixel | [x] |
| 14 | `unfilter` | first row filter 1; multiple pixels | [x] |
| 15 | `unfilter` | first row filter 2; zero-length row | [x] |
| 16 | `unfilter` | first row filter 2; one pixel | [x] |
| 17 | `unfilter` | first row filter 2; multiple pixels | [x] |
| 18 | `unfilter` | first row filter 3; zero-length row | [x] |
| 19 | `unfilter` | first row filter 3; one pixel | [x] |
| 20 | `unfilter` | first row filter 3; multiple pixels | [x] |
| 21 | `unfilter` | first row filter 4; zero-length row | [x] |
| 22 | `unfilter` | first row filter 4; one pixel | [x] |
| 23 | `unfilter` | first row filter 4; multiple pixels | [x] |
| 24 | `unfilter` | multi-row, first filter 0, later filter 0, multiple pixels | [x] |
| 25 | `unfilter` | multi-row, first filter 0, later filter 1, multiple pixels | [x] |
| 26 | `unfilter` | multi-row, first filter 0, later filter 2, multiple pixels | [x] |
| 27 | `unfilter` | multi-row, first filter 0, later filter 3, multiple pixels | [x] |
| 28 | `unfilter` | multi-row, first filter 0, later filter 4, multiple pixels | [x] |
| 29 | `unfilter` | multi-row, first filter 1, later filter 0, multiple pixels | [x] |
| 30 | `unfilter` | multi-row, first filter 1, later filter 1, multiple pixels | [x] |
| 31 | `unfilter` | multi-row, first filter 1, later filter 2, multiple pixels | [x] |
| 32 | `unfilter` | multi-row, first filter 1, later filter 3, multiple pixels | [x] |
| 33 | `unfilter` | multi-row, first filter 1, later filter 4, multiple pixels | [x] |
| 34 | `unfilter` | multi-row, first filter 2, later filter 0, multiple pixels | [x] |
| 35 | `unfilter` | multi-row, first filter 2, later filter 1, multiple pixels | [x] |
| 36 | `unfilter` | multi-row, first filter 2, later filter 2, multiple pixels | [x] |
| 37 | `unfilter` | multi-row, first filter 2, later filter 3, multiple pixels | [x] |
| 38 | `unfilter` | multi-row, first filter 2, later filter 4, multiple pixels | [x] |
| 39 | `unfilter` | multi-row, first filter 3, later filter 0, multiple pixels | [x] |
| 40 | `unfilter` | multi-row, first filter 3, later filter 1, multiple pixels | [x] |
| 41 | `unfilter` | multi-row, first filter 3, later filter 2, multiple pixels | [x] |
| 42 | `unfilter` | multi-row, first filter 3, later filter 3, multiple pixels | [x] |
| 43 | `unfilter` | multi-row, first filter 3, later filter 4, multiple pixels | [x] |
| 44 | `unfilter` | multi-row, first filter 4, later filter 0, multiple pixels | [x] |
| 45 | `unfilter` | multi-row, first filter 4, later filter 1, multiple pixels | [x] |
| 46 | `unfilter` | multi-row, first filter 4, later filter 2, multiple pixels | [x] |
| 47 | `unfilter` | multi-row, first filter 4, later filter 3, multiple pixels | [x] |
| 48 | `unfilter` | multi-row, first filter 4, later filter 4, multiple pixels | [x] |
| 49 | `unfilter` | multi-row filters 0..4; one-pixel rows (`len == bpp`) | [x] |
| 50 | `unfilter` | multi-row filters 0..4; `bpp > 1` channel-boundary loops | [x] |
| 51 | `cp_inflate` | one stored block, empty payload, all input alignments, exact output | [x] |
| 52 | `cp_inflate` | one stored block, randomized nonempty payload, all alignments, exact output | [x] |
| 53 | `cp_inflate` | one stored block, randomized nonempty payload, all alignments, spare output | [x] |
| 54 | `cp_inflate` | multiple stored blocks (`bfinal` 0 then 1), randomized payloads | [x] |
| 55 | `cp_inflate` | one fixed-Huffman block containing only end-of-block; zero output | [x] |
| 56 | `cp_inflate` | fixed-Huffman literal-only block, randomized bytes, exact output | [x] |
| 57 | `cp_inflate` | fixed-Huffman literal-only block, randomized bytes, spare output | [x] |
| 58 | `cp_inflate` | fixed-Huffman length/distance with backwards distance 1 (`memset` branch) | [x] |
| 59 | `cp_inflate` | fixed-Huffman length/distance with backwards distance greater than 1 (copy loop) | [x] |
| 60 | `cp_inflate` | fixed-Huffman length and distance codes spanning zero/nonzero extra-bit widths | [x] |
| 61 | `cp_inflate` | dynamic-Huffman block from randomized high-entropy data; all input alignments | [x] |
| 62 | `cp_inflate` | dynamic-Huffman block from repetitive data, exercising compact code-length trees | [x] |
| 63 | `cp_inflate` | multi-block stream combining block-final false/true paths | [x] |
| 64 | `unfilter` | multi-row `w == 0`, `bpp > 0`; later filter bytes overlap the loops C still executes | [x] |

There are no Cargo features in `Cargo.toml`; the feature matrix therefore has
one semantic combination, verified both normally and with
`--no-default-features`.
