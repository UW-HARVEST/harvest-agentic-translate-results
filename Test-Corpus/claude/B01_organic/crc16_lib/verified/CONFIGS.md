# CONFIGS.md — Configuration-surface table (Phase B gate)

## Derivation method

The public API is the single declaration in `c_src/include/lib.h`:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

There is **one** public entry point and it *is* the lowest-level entry point —
`c_src` exposes no convenience wrapper and no one-shot/streaming split, so
"exercise the low-level entry points, not only the wrappers" reduces to driving
`crc16` directly (which is also how the upstream consumer, tflac, uses it).

There are no runtime option/mode/flag setters (no init struct, no context, no
global state), and no `#if`/`#ifdef` in `c_src`. The axes the C **actually
branches on** are therefore exactly the ones visible in its two loops:

| axis | source of the branch | values that matter |
|---|---|---|
| A. `len` vs. the block/tail split | `while (len >= 8)` then `while (len--)` | `0`; `1..7` (tail only); `8` (block, empty tail); `9..15` (block + 1..7 tail); `16`, `24` (multi-block); multi-block + tail; large |
| B. seed `crc` | `crc16 >> 8`, `crc16 & 0xFF` (block indices), `crc16 << 8` truncation + `(crc16 >> 8) ^ byte` (tail index) | `0x0000`, `0xFFFF`, `0x00FF`, `0xFF00`, `0x0001`, random |
| C. data byte pattern | `d[0]<<8\|d[1]`, `d[2]..d[7]` and `*d++` used as table indices | all-`0x00`, all-`0xFF`, `0..255` ramp (covers every one of the 256 index slots), random, alternating |
| D. call composition | the function is resumable: chaining feeds the previous return in as the next seed | one-shot; chained at 8-aligned splits; chained at arbitrary (unaligned) splits; byte-at-a-time |
| E. buffer offset of `d` | byte-wise reads, so any offset is legal | offset `0`, `1`, `3`, `7` inside a larger allocation |
| F. which internal path computes a given byte | block loop (slice-by-8) vs. tail loop must agree | same data via `len=8` vs. 8× `len=1` |

Empirically verified against the C `.so` before writing tests (so the rows below
assert the C's real behaviour, not assumed behaviour):

* arbitrary-split chaining == one-shot: **True** (200 random cases)
* block path == chained tail path: **True** (500 random cases)
* `crc16(NULL, 0, 0xBEEF)` == `0xBEEF`, no crash
* `crc16("123456789", 9, 0)` == `0xFEE8`

Every row is driven with **many randomized inputs** from a fixed seed
(`StdRng`-free deterministic xorshift, seed `0x2545F4914F6CDD1D`) unless the row
is inherently a single shape.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `crc16` | `len = 0`, valid pointer, seeds {0x0000, 0xFFFF, 0x00FF, 0xFF00, random×64} — neither loop runs | `c1_len_zero_all_seeds` | [x] |
| C2 | `crc16` | `len = 1..7` (tail-only path, every tail count), random data × random seeds, 256 cases per length | `c2_tail_only_lengths_1_to_7` | [x] |
| C3 | `crc16` | `len = 8` exactly (one block, tail loop runs zero times), random data × random seeds × 512 | `c3_exactly_one_block` | [x] |
| C4 | `crc16` | `len = 9..15` (one block + 1..7 tail bytes), random × 256 per length | `c4_one_block_plus_tail` | [x] |
| C5 | `crc16` | `len = 16, 24, 32, 64` (multi-block, no tail), random × 256 per length | `c5_multi_block_no_tail` | [x] |
| C6 | `crc16` | `len` = multi-block **plus** tail (17..71, all residues mod 8), random × 256 per length | `c6_multi_block_plus_tail` | [x] |
| C7 | `crc16` | all-`0x00` data, every length `0..=72`, seeds {0, 0xFFFF, 0x1234} | `c7_all_zero_bytes` | [x] |
| C8 | `crc16` | all-`0xFF` data, every length `0..=72`, seeds {0, 0xFFFF, 0x1234} | `c8_all_ff_bytes` | [x] |
| C9 | `crc16` | `0..255` ramp data (hits all 256 slots of all 8 tables), every length `0..=256`, seeds {0, 0xFFFF} | `c9_full_byte_ramp_all_lengths` | [x] |
| C10 | `crc16` | seed boundary sweep: **all 65536** seeds over a fixed 8-byte and a fixed 3-byte buffer | `c10_exhaustive_seed_sweep` | [x] |
| C11 | `crc16` | single byte, **all 256** byte values × seeds {0x0000, 0xFFFF, 0xABCD} (tail index `(crc>>8)^b` boundary) | `c11_exhaustive_single_byte` | [x] |
| C12 | `crc16` | chained calls at **8-aligned** splits vs. one-shot; both C and Rust chained, compared to each other | `c12_chained_aligned_splits` | [x] |
| C13 | `crc16` | chained calls at **arbitrary/unaligned** splits (random split points, 1–5 chunks) | `c13_chained_unaligned_splits` | [x] |
| C14 | `crc16` | byte-at-a-time chaining (`len=1` × N) — forces tail path only, compared against C doing the same | `c14_byte_at_a_time_chaining` | [x] |
| C15 | `crc16` | block path vs. tail path cross-check: `len=8k` one-shot compared against 8k× `len=1` chained, on **both** libraries | `c15_block_vs_tail_paths_agree` | [x] |
| C16 | `crc16` | unaligned `d` (offset 0,1,2,3,5,7 into a larger allocation) × lengths {0..24} × random data | `c16_unaligned_buffer_offsets` | [x] |
| C17 | `crc16` | large buffers: 1 KiB, 64 KiB, 1 MiB random data, seeds {0, 0xFFFF} | `c17_large_buffers` | [x] |
| C18 | `crc16` | fully randomized fuzz: 20 000 iterations of random `len` ∈ 0..=1024, random bytes, random seed | `c18_randomized_fuzz` | [x] |
| C19 | `crc16` | `len` shorter than the allocation (caller under-reports) — trailing bytes must be ignored identically | `c19_len_shorter_than_buffer` | [x] |
| C20 | `crc16` | alternating / structured patterns (`0x00 0xFF`, `0xFF 0x00`, `0xAA 0x55`, ASCII text) over lengths 0..=64 | `c20_structured_patterns` | [x] |
