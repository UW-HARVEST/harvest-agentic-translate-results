# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, no optional dependencies, and no
default feature declaration. `c_src/CMakeLists.txt` has no options or
conditional sources, and the C source has no preprocessor configuration
branches. There is exactly one valid build combination:

| # | Rust invocation | CMake configuration | [ ] |
|---|-----------------|---------------------|-----|
| 1 | `--no-default-features` (no named features) | default, with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime Configurations

The only public entry point is `dequantize_granule`. `scf`, `stereo_bands`,
and `scfcod` are not read by this C function and therefore add no branch axis.
Input bytes are read most-significant-bit first. The table cross-products the
actual loop, allocation, byte-crossing, and bit-limit branches while pruning
values that execute the same C path.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dequantize_granule` | `total_bands = 0`, `group_size = 0`: empty bands and empty groups; no input/output access, return 0 | [x] |
| 2 | `dequantize_granule` | `total_bands = 0`, positive `group_size`: empty bands with one/many group elements; no bit reads or output writes | [x] |
| 3 | `dequantize_granule` | `total_bands = 1..32`, `group_size = 0`, all visited `bitalloc = 0..16`: direct/zero allocations perform no reads because the inner sample loop is empty | [x] |
| 4 | `dequantize_granule` | `total_bands = 1..32`, `group_size = 0`, a visited `bitalloc = 17..21`: grouped allocation reads one packed code per band/group even though it writes no samples | [x] |
| 5 | `dequantize_granule` | One and many bands, positive group size, all visited `bitalloc = 0`: allocation skip branch; output remains untouched | [x] |
| 6 | `dequantize_granule` | Direct quantizer `bitalloc = 1`, aligned and unaligned initial `bs.pos`: one-bit reads and `half = 0` boundary | [x] |
| 7 | `dequantize_granule` | Direct quantizer `bitalloc = 2..7`, aligned/unaligned positions: reads remain in or cross a byte | [x] |
| 8 | `dequantize_granule` | Direct quantizer `bitalloc = 8`, aligned/unaligned positions: exact-byte and two-byte reads | [x] |
| 9 | `dequantize_granule` | Direct quantizer `bitalloc = 9..15`, aligned/unaligned positions: two/three-byte reads | [x] |
| 10 | `dequantize_granule` | Direct quantizer `bitalloc = 16`, aligned/unaligned positions: upper direct-width boundary and multi-byte reads | [x] |
| 11 | `dequantize_granule` | Grouped quantizer `bitalloc = 17`: modulus 3, packed width 5, one/many output digits | [x] |
| 12 | `dequantize_granule` | Grouped quantizer `bitalloc = 18`: modulus 5, packed width 7, one/many output digits | [x] |
| 13 | `dequantize_granule` | Grouped quantizer `bitalloc = 19`: modulus 9, packed width 10, byte-crossing packed reads | [x] |
| 14 | `dequantize_granule` | Grouped quantizer `bitalloc = 20`: modulus 17, packed width 17, multi-byte packed reads | [x] |
| 15 | `dequantize_granule` | Grouped quantizer `bitalloc = 21`: modulus 33, packed width 31, largest width that avoids an invalid C shift in `get_bits` | [x] |
| 16 | `dequantize_granule` | Mixed sparse allocations (`0`, direct `1..16`, grouped `17..21`), `total_bands = 2..32`, one/many group elements: alternating channel offset and 18-sample band stride | [x] |
| 17 | `dequantize_granule` | Positive `group_size` shapes 1, 3, 12, and 19 with sufficiently large output storage: one/many loops and overlapping writes when size exceeds the 18-sample stride | [x] |
| 18 | `dequantize_granule` | Exact bit limit (`pos + n == limit`) for direct and grouped reads: accepted boundary | [x] |
