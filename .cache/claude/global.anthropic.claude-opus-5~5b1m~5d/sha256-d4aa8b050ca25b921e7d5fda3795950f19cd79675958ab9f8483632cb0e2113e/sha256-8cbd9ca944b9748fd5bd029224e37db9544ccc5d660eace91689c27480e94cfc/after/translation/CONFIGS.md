# CONFIGS.md — configuration-surface table (valid inputs)

## Axes derived from the C source

`c_src/src/lib.c` is a single function with **no** options, modes, flags,
`switch`es or `#ifdef`s. The only things it branches on are:

* **A. length class of `len`** (`while (len >= 8)` and `while (len--)`):
  * `len == 0` → no loop body at all
  * `1 <= len <= 7` → tail loop only
  * `len == 8` → one fast iteration, no tail
  * `len % 8 == 0`, `len > 8` → fast loop only, many iterations
  * `len % 8 != 0`, `len > 8` → fast loop + 1..7 tail iterations
  * large `len` (≥ 2^16, up to 2^20) → many fast iterations
* **B. seed value `crc16`** (used as `crc >> 8`, `crc & 0xFF`, `crc << 8`):
  `0x0000`, `0xFFFF`, high-byte-only `0xFF00`, low-byte-only `0x00FF`,
  random 16-bit values.
* **C. data byte values** (used as table indices 0..255 in eight tables):
  all-`0x00`, all-`0xFF`, incrementing `0..255`, random.
* **D. pointer offset / alignment** of `d` (0..8 bytes into a buffer): the C
  code does unaligned `d[0]..d[7]` byte reads, so alignment must not matter.
* **E. entry points**: the *entire* public API is the single lowest-level
  function `crc16` (there is no wrapper/one-shot layer). Incremental use —
  feeding the output of one call as the seed of the next, i.e. chunked
  streaming — is the composed-pipeline usage a real consumer performs, so it
  is a distinct axis.
  * one-shot over the whole buffer
  * chunked: split the buffer at arbitrary boundaries and chain the seed
    (chunk boundaries both multiples of 8 and non-multiples)

Cross-product pruned to the combinations the code actually distinguishes:

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `crc16` | `len = 0`, seed = 0, valid non-null `d` | [x] |
| 2  | `crc16` | `len = 0`, seed = each of {0, 0xFFFF, 0xFF00, 0x00FF, 256 random} | [x] |
| 3  | `crc16` | `len = 1..7` (tail-only), random data, seed = 0 | [x] |
| 4  | `crc16` | `len = 1..7` (tail-only), random data, random seeds (many) | [x] |
| 5  | `crc16` | `len = 1..7`, data all `0x00` and all `0xFF`, seeds {0, 0xFFFF, 0xFF00, 0x00FF} | [x] |
| 6  | `crc16` | `len = 8` exactly (single fast iteration), random data, random seeds | [x] |
| 7  | `crc16` | `len = 8`, data all `0x00`, all `0xFF`, and `0..7` incrementing; extreme seeds | [x] |
| 8  | `crc16` | `len = 9..15` (fast + 1..7 tail), random data, random seeds | [x] |
| 9  | `crc16` | `len` multiple of 8 in 16..1024, random data, random seeds | [x] |
| 10 | `crc16` | `len` non-multiple of 8 in 16..1024, random data, random seeds | [x] |
| 11 | `crc16` | `len = 65535 / 65536 / 65537` (≥ 2^16 boundary), random data, random seed | [x] |
| 12 | `crc16` | `len = 1 << 20` (1 MiB), random data, seeds {0, 0xFFFF} | [x] |
| 13 | `crc16` | data = all 256 single-byte values, `len = 1`, seed sweep over all 256 high-byte values | [x] |
| 14 | `crc16` | 8-byte group where exactly one position holds `0xFF` and the rest `0x00`, for each of the 8 positions (drives each of the 8 tables at index 255) | [x] |
| 15 | `crc16` | unaligned `d`: offsets 0..8 into a buffer, `len` fixed 64 and 67, random data/seed | [x] |
| 16 | `crc16` (chained) | chunked streaming, chunk sizes all multiples of 8, total 1..2048, output-as-seed | [x] |
| 17 | `crc16` (chained) | chunked streaming, arbitrary (non-multiple-of-8) chunk sizes, total 1..2048 | [x] |
| 18 | `crc16` (chained) | chunked with zero-length chunks interleaved (identity chunks) | [x] |
| 19 | `crc16` | randomized fuzz sweep: 20 000 cases, `len` 0..600, random data, random seed (fixed seed PRNG) | [x] |
| 20 | `crc16` | table-coverage sweep: every table index 0..255 reached in tables 0..7 via crafted 8-byte groups + seeds | [x] |

All rows are exercised in `tests/differential.rs` (module `configs`) against
BOTH `.so` files loaded with `libloading`, with a fixed-seed xorshift PRNG for
reproducibility.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
build configuration is the default one (`cargo test`,
`cargo test --no-default-features`); both are run by `run_tests.sh`.
