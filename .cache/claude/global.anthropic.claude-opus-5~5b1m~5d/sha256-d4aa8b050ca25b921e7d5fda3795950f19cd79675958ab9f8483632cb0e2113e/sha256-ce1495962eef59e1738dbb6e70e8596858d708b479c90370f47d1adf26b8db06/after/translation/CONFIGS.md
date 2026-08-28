# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Derived **mechanically** from what the C code actually branches on, the same way
`ERRORS.md` is derived. Not a guess at which configurations "matter".

## Axis derivation from the C source

`c_src/src/lib.c` in full (20 lines) contains exactly two branches:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16) {
    while (len >= 8) {                                   /* branch 1 */
        crc16 ^= d[0] << 8 | d[1];
        crc16 = tflac_crc16_tables[7][crc16 >> 8] ^
                tflac_crc16_tables[6][crc16 & 0xFF] ^
                tflac_crc16_tables[5][d[2]] ^ tflac_crc16_tables[4][d[3]] ^
                tflac_crc16_tables[3][d[4]] ^ tflac_crc16_tables[2][d[5]] ^
                tflac_crc16_tables[1][d[6]] ^ tflac_crc16_tables[0][d[7]];
        d += 8;
        len -= 8;
    }
    while (len--) {                                      /* branch 2 */
        crc16 = (crc16 << 8) ^ tflac_crc16_tables[0][(crc16 >> 8) ^ *d++];
    }
    return crc16;
}
```

### Runtime options / modes / flags: **none**

`grep -nE '^\s*#' ` over both C files yields only the two `#include`s — there are
**no** `#ifdef`s, no compile-time options, no feature macros. `grep` for
`enum|switch|case|static.*=|global` yields nothing settable: there is no
configuration struct, no init function, no mode setter, no byte-order handling,
no state object. The function is pure and stateless; its entire configuration is
its three arguments. `translation/Cargo.toml` correspondingly has **no
`[features]` section**, so there is exactly one feature combination (the empty
default) — see Phase D.

### Public entry points: **one, and it is already the lowest level**

The header declares exactly one function (`lib.h:282`). There is no
convenience/one-shot wrapper layered over a lower-level API — `crc16` *is* the
low-level primitive. So "exercise the low-level entry points directly, not only
the convenience wrappers" is satisfied by construction; the extra rigor is moved
to the **composed-pipeline** rows (13–18), which drive `crc16` the way its real
consumer (tflac, encoding a FLAC frame) does: repeated calls chaining the
previous result in as the next seed.

### Axes the code genuinely distinguishes

| axis | why it is an axis (source evidence) | values to cover |
|------|--------------------------------------|-----------------|
| **A. `len` vs the slice-by-8 threshold** | `while (len >= 8)` — selects whether the wide path runs at all | `0`; `1..7` (tail only); `8` (wide ×1, no tail); `9..15` (wide ×1 + tail); `16`, `24`, … (wide ×N); large |
| **B. `len % 8` residue** | `len -= 8` leaves `len % 8` for `while (len--)`; tail runs 0–7 times | all 8 residues `0..=7`, at several quotients |
| **C. number of wide iterations** | loop is iterated, so state carries between rounds | 0, 1, 2, many (state-carry bugs need ≥2) |
| **D. seed `crc16`** | feeds `crc16 >> 8` (table 7 index), `crc16 & 0xFF` (table 6 index) and `crc16 << 8` (truncating) | `0x0000`, `0xFFFF`, byte-boundary values, random over full `u16` |
| **E. byte value per lane** | the 8 bytes of each group index **8 different tables** (`d[0]`,`d[1]` via the seed XOR; `d[2]`→T5, `d[3]`→T4, `d[4]`→T3, `d[5]`→T2, `d[6]`→T1, `d[7]`→T0) — a lane/table mix-up is invisible unless each lane is varied independently | `0x00`, `0xFF`, all `0..=255`, random |
| **F. buffer content shape** | value-dependent table indexing | all-zero, all-`0xFF`, incrementing, single-bit, random |
| **G. call composition (streaming)** | `crc` is both input and output → the primitive is designed to be chained; a wrong tail/wide interaction only shows when a message is split at a non-multiple of 8 | one-shot vs split at every offset |
| **H. buffer alignment / offset** | C reads `d[0..8]` bytewise; Rust builds a slice. An alignment or slice-origin bug shows only at unaligned starts | offsets `0..=7` into an over-allocated buffer |

Rows below are the pruned cross-product of A–H: one row per combination the C
code treats differently. Every row is driven with **many randomized inputs
(fixed seed `0x5EED_C0DE_D00D_F00D`, `SplitMix64`)**, not one hand-picked value,
and compares C vs Rust byte-for-byte through the `.so` exports.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|------------------------------------------|------|-----|
| 1 | `crc16` | **A=0**: `len == 0`, valid ptr; × all 65536 seeds (exhaustive). Wide loop 0×, tail 0×. | `cfg01_len_zero_all_seeds` | [x] |
| 2 | `crc16` | **A=tail-only, B=1..7, C=0**: `len ∈ 1..=7` × random content × random seeds. Tail loop only. | `cfg02_tail_only_lengths` | [x] |
| 3 | `crc16` | **A=8, C=1, B=0**: `len == 8` exactly. Wide 1×, tail 0×. Random content, random seeds. | `cfg03_exactly_one_wide_block` | [x] |
| 4 | `crc16` | **C=1, B=1..7**: `len ∈ 9..=15`. Wide 1× then tail 1–7×. Exercises the wide→tail handoff. | `cfg04_one_wide_block_plus_tail` | [x] |
| 5 | `crc16` | **C=2, B=0**: `len == 16`. Two wide rounds, no tail — state must carry between rounds. | `cfg05_two_wide_blocks_no_tail` | [x] |
| 6 | `crc16` | **C=many, B=0**: `len ∈ {24,32,64,128,256,1024,4096}` (multiples of 8). Wide only. | `cfg06_many_wide_blocks_no_tail` | [x] |
| 7 | `crc16` | **C=many, B=1..7 (full sweep)**: every `len` in `0..=520` — covers all 8 residues at 65 quotients, i.e. the whole A×B×C grid densely. Random content + random seed per length. | `cfg07_dense_length_sweep_0_to_520` | [x] |
| 8 | `crc16` | **E, exhaustive per lane**: for each lane `i ∈ 0..8` and each value `v ∈ 0..=255`, an 8-byte block with `d[i] = v` and the other 7 bytes fixed. 2048 combinations. Pins every one of the 8 tables to its lane. | `cfg08_every_byte_value_in_every_lane` | [x] |
| 9 | `crc16` | **E, exhaustive tail table**: `len == 1` with byte `0..=255` × seeds sweeping `crc >> 8` over `0..=255`. Pins all 256 entries of `tables[0]` used by the tail loop. | `cfg09_tail_table_exhaustive` | [x] |
| 10 | `crc16` | **D extremes**: seeds `0x0000,0x0001,0x00FF,0x0100,0x7FFF,0x8000,0xFF00,0xFEFF,0xFFFE,0xFFFF` × lengths `0..=24` × random content. Pins the `crc << 8` truncation and both seed-derived table indices. | `cfg10_seed_extremes_across_lengths` | [x] |
| 11 | `crc16` | **F degenerate content**: all-`0x00`, all-`0xFF`, incrementing `i as u8`, decrementing, alternating `0x00/0xFF`, single-bit-set at every bit position; lengths `0..=64`. | `cfg11_degenerate_content_patterns` | [x] |
| 12 | `crc16` | **F random, property-style**: 20 000 trials, random `len ∈ 0..=1024`, random bytes, random seed. The broad randomized sweep. | `cfg12_property_random_full_range` | [x] |
| 13 | `crc16` | **G composed pipeline, split at 1**: feed a 512-byte message one byte at a time, chaining `crc`. Forces 512 tail-only calls. Compared C-chain vs Rust-chain **and** vs the one-shot value. | `cfg13_stream_one_byte_at_a_time` | [x] |
| 14 | `crc16` | **G composed pipeline, split at 8**: same message in 8-byte chunks — all wide, no tail, chained. | `cfg14_stream_eight_byte_chunks` | [x] |
| 15 | `crc16` | **G composed pipeline, split at every offset**: for every split point `k ∈ 0..=n`, `crc16(tail, crc16(head))` vs one-shot. Catches wide/tail interaction bugs at all 8 residues. | `cfg15_stream_split_at_every_offset` | [x] |
| 16 | `crc16` | **G composed pipeline, random chunking**: random 3-way and N-way splits with random chunk sizes, 2000 trials, chained through both `.so`s. | `cfg16_stream_random_chunk_sequences` | [x] |
| 17 | `crc16` | **H alignment/offset**: identical logical bytes read starting at offsets `0..=7` inside an over-allocated buffer, lengths `0..=64`. Detects slice-origin/alignment assumptions the bytewise C code does not make. | `cfg17_unaligned_start_offsets` | [x] |
| 18 | `crc16` | **A large, no `len` truncation**: `len ∈ {0xFFFF, 0x1_0000, 0x1_0001, 0x1_0007, 65536+8}` on a 64 KiB+ buffer — `len` above the `u16` range, exercising the wrapper's `len as usize` widening. | `cfg18_large_buffers_beyond_u16` | [x] |

**Rows: 18. Unchecked: 0.**

## Finding: the wide path and the tail path are observationally equivalent

While mutation-testing the suite (`./mutation_check.sh`), the mutation
`while len >= 8` -> `while len > 8` **survived**. Investigation showed this is an
*equivalent mutant*, not a gap in the tests: the slice-by-8 wide step computes
exactly the same CRC as 8 consecutive byte-at-a-time tail steps. That identity is
the entire point of the slice-by-8 construction, and it was confirmed **against
the C `.so` itself** — one 8-byte call vs eight chained 1-byte calls agreed on
200 000/200 000 random (block, seed) pairs.

Consequences, both of which are properties of the C code that the Rust must and
does share:

* The wide-loop threshold is a free parameter for any value `>= 8`; no input can
  distinguish `>= 8` from `> 8` from `>= 16`. A *below-block-size* threshold
  (`>= 7`) is genuinely broken and is caught.
* Row 15 (`cfg15_stream_split_at_every_offset`) is what pins this identity down
  in the suite: it asserts `crc16(tail, crc16(head, seed)) == crc16(whole, seed)`
  for every split point, which is precisely the wide/tail interchange law.

`mutation_check.sh` therefore substitutes the genuinely non-equivalent
off-by-ones (block step `len -= 8`, cursor step `pos += 8`, and a sub-block
threshold) for the equivalent one. Final result: **17 mutations injected, 17
caught, 0 survived.**
