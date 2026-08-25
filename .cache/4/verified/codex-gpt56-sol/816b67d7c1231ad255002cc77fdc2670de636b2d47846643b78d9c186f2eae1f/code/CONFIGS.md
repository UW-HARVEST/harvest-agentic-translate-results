# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and no default features.
`c_src/CMakeLists.txt` declares no options, cache variables, compile
definitions, or conditional sources. There is exactly one valid feature
combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default, with position-independent code enabled for the shared library | [x] |

## Runtime Configurations

The sole public entry point is `pinflate(void *in, int in_bytes, void *out,
int out_bytes)`. The rows below are the source-level branch cross-product
pruned where branches have identical externally observable behavior. Every
row is exercised for input-address alignments 0, 1, 2, and 3 modulo 4 and
input tail lengths 0, 1, 2, and 3 modulo 4 whenever padding can vary without
changing the represented stream.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pinflate` | Final stored block (`BFINAL=1`, `BTYPE=0`), empty payload, exact output capacity | [x] |
| 2 | `pinflate` | Final stored block, nonempty randomized payload, exact output capacity; `LEN` and `NLEN` complementary and remaining input bytes exactly `LEN` | [x] |
| 3 | `pinflate` | Final stored block, nonempty randomized payload, spare output capacity (stored path does not inspect capacity) | [x] |
| 4 | `pinflate` | Final fixed-Huffman block (`BTYPE=1`), empty output (EOB only) | [x] |
| 5 | `pinflate` | Fixed-Huffman block with randomized literal symbols and exact output capacity | [x] |
| 6 | `pinflate` | Fixed-Huffman block with literal symbols and spare output capacity | [x] |
| 7 | `pinflate` | Fixed-Huffman back-reference with `backwards_distance == 1` (`memset` branch) | [x] |
| 8 | `pinflate` | Fixed-Huffman back-reference with `backwards_distance > 1` (byte-copy loop), no extra distance bits | [x] |
| 9 | `pinflate` | Fixed-Huffman back-reference using a length code with nonzero extra bits | [x] |
| 10 | `pinflate` | Fixed-Huffman back-reference using a distance code with nonzero extra bits | [x] |
| 11 | `pinflate` | Final dynamic-Huffman block (`BTYPE=2`) with randomized incompressible payload dominated by literals | [x] |
| 12 | `pinflate` | Dynamic header decodes code-length symbol 16 (repeat previous length 3-6 times) | [x] |
| 13 | `pinflate` | Dynamic header decodes code-length symbol 17 (repeat zero 3-10 times) | [x] |
| 14 | `pinflate` | Dynamic header decodes code-length symbol 18 (repeat zero 11-138 times) | [x] |
| 15 | `pinflate` | Dynamic literal/distance tree contains zero-length entries and nonzero entries of length `<= 9`, exercising lookup construction and null-state distance-tree construction | [x] |
| 16 | `pinflate` | Dynamic literal/distance tree contains a code length `> 9`, exercising binary tree decode without a lookup entry | [x] |
| 17 | `pinflate` | Compressed stream emits literal (`symbol < 256`), end marker (`symbol == 256`), and length/distance (`symbol > 256`) in one operation | [x] |
| 18 | `pinflate` | Multiple blocks (`BFINAL=0` followed by `BFINAL=1`) with fixed/dynamic compressed block transitions | [x] |
| 19 | `pinflate` | Multiple compressed blocks including byte-alignment changes across block boundaries | [x] |
| 20 | `pinflate` | Input setup matrix: address alignment 0-3 modulo 4 crossed with final partial-word byte count 0-3; randomized valid fixed and dynamic streams | [x] |
| 21 | `pinflate` | Boundary-valid output: final literal lands exactly at `out_end` | [x] |
| 22 | `pinflate` | Boundary-valid output: final back-reference lands exactly at `out_end` | [x] |
| 23 | `pinflate` | Large valid streams crossing 32-bit input words and DEFLATE's 32 KiB maximum distance range | [x] |

The C source has no runtime option struct, mode setter, byte-order option,
element type, or format selector. Input is always a raw DEFLATE bitstream and
output is always bytes.
