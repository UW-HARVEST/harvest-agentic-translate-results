# Configuration-surface table

The public API consists only of `pinflate`. The rows below are the
cross-product pruned to branches the C implementation actually distinguishes:
block type, block sequencing/final-bit behavior, input address alignment and
word/tail loading, stored-block byte alignment, Huffman table form, dynamic
code-length repeat symbols, literal versus string output, distance-one versus
general copying, output capacity, and empty/one/many output shapes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pinflate` | final stored block (`BTYPE=0`), empty payload, exact zero output capacity | [x] |
| 2 | `pinflate` | final stored block, one-byte payload, exact output capacity | [x] |
| 3 | `pinflate` | final stored block, many-byte randomized payload, exact output capacity | [x] |
| 4 | `pinflate` | final stored block, randomized payload, larger-than-needed output capacity | [x] |
| 5 | `pinflate` | stored block entered with a non-byte-aligned bit count so `cp_stored` discards padding | [x] |
| 6 | `pinflate` | fixed Huffman (`BTYPE=1`), empty output (end-of-block only) | [x] |
| 7 | `pinflate` | fixed Huffman, randomized literal-only output | [x] |
| 8 | `pinflate` | fixed Huffman, length/distance string with distance 1 (`memset` branch) | [x] |
| 9 | `pinflate` | fixed Huffman, length/distance string with distance greater than 1 (byte-copy branch) | [x] |
| 10 | `pinflate` | fixed Huffman, length and distance symbols requiring extra bits, including boundary bases | [x] |
| 11 | `pinflate` | dynamic Huffman (`BTYPE=2`), direct code-length symbols (default switch arm) | [x] |
| 12 | `pinflate` | dynamic Huffman code lengths using repeat symbol 16 | [x] |
| 13 | `pinflate` | dynamic Huffman code lengths using zero-repeat symbol 17 | [x] |
| 14 | `pinflate` | dynamic Huffman code lengths using long zero-repeat symbol 18 | [x] |
| 15 | `pinflate` | dynamic Huffman payload with randomized literals and length/distance copies | [x] |
| 16 | `pinflate` | multiple non-final/final compressed blocks, exercising the outer block loop | [x] |
| 17 | `pinflate` | input pointer aligned to 4 bytes (`first_bytes=0`) | [x] |
| 18 | `pinflate` | input pointer offset so `first_bytes=1` | [x] |
| 19 | `pinflate` | input pointer offset so `first_bytes=2` | [x] |
| 20 | `pinflate` | input pointer offset so `first_bytes=3` | [x] |
| 21 | `pinflate` | post-prefix input has no trailing partial word (`last_bytes=0`) | [x] |
| 22 | `pinflate` | post-prefix input has one trailing byte (`last_bytes=1`) | [x] |
| 23 | `pinflate` | post-prefix input has two trailing bytes (`last_bytes=2`) | [x] |
| 24 | `pinflate` | post-prefix input has three trailing bytes (`last_bytes=3`) | [x] |

There are no runtime options, modes, flags, public low-level entry points,
Cargo features, conditional C compilation branches, or executable drivers in
this project.

Rows 1–10 and 16–24 are exercised by
`phase_b_stored_fixed_and_shape_rows`; rows 11–15 are exercised by
`phase_b_dynamic_rows`. Each group calls both shared objects through
`libloading`.
