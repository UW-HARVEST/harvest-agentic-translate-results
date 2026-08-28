# CONFIGS.md — Phase A: configuration / valid-input surface table

## Public entry points (complete)

`c_src/include/lib.h` declares exactly one function, and it is the lowest-level entry point
there is — there is no convenience wrapper layer to hide behind:

| entry point | signature | linkage |
|---|---|---|
| `hdr_compare` | `int hdr_compare(const uint8_t *h1, const uint8_t *h2)` | exported (`T`) |
| `hdr_valid` | `static int hdr_valid(const uint8_t *h)` | internal; reachable **only** through `hdr_compare`, exercised by every row below |

## Axes the C actually branches on

Derived from the `&&`/`||` operands in `c_src/src/lib.c` (there are no `#ifdef`s, no globals,
no runtime option setters, no state — the "options" of this API are the packed bit-fields of
the two 3-byte headers, so those bit-fields *are* the configuration axes):

| axis | field | C expression | distinguished values |
|---|---|---|---|
| A | `h2` sync byte | `h[0] == 0xff` | `0xFF` / anything else |
| B | `h2[1]` sync-class | `(h[1] & 0xF0) == 0xF0` \|\| `(h[1] & 0xFE) == 0xE2` | class **A** = `0xF0..0xFF` (12 valid), class **B** = `{0xE2,0xE3}`, class **none** |
| C | layer index | `((h[1] >> 1) & 3) != 0` | `1`, `2`, `3` valid; `0` reserved |
| D | CRC/protection bit | masked out by `0xFE` in row-G check | `h[1] & 1 ∈ {0,1}` — must be *ignorable* |
| E | bitrate index | `(h[2] >> 4) != 15` and `(h[2] & 0xF0) == 0` | `0` = free-format, `1..14` = normal, `15` = bad |
| F | sample-rate index | `((h[2] >> 2) & 3) != 3` | `0`, `1`, `2` valid; `3` reserved |
| G | padding+private bits | never inspected (`h[2] & 0x03`) | `0..3` — must be *ignorable* |
| H | `h1[1]` vs `h2[1]` | `((h1[1] ^ h2[1]) & 0xFE) == 0` | identical / differ in bit 0 only / differ in bits 1..7 |
| I | `h1[2]` vs `h2[2]` sample-rate | `((h1[2] ^ h2[2]) & 0x0C) == 0` | equal / differ |
| J | free-format agreement | `!(((h1[2]&0xF0)==0) ^ ((h2[2]&0xF0)==0))` | both free / both non-free / exactly one free |
| K | `h1[0]` | never read | arbitrary, incl. `0x00` and `0xFF` — must not affect the result |
| L | pointer shape | — | two distinct buffers / `h1 == h2` (aliased) / overlapping views / page-end-guarded / unaligned |

Valid `h2[1]` set (axes B+C): `{0xE2,0xE3,0xF2,0xF3,0xF4,0xF5,0xF6,0xF7,0xFA,0xFB,0xFC,0xFD,0xFE,0xFF}` — 14 values.
Valid `h2[2]` set (axes E+F): the 180 values in `0x00..=0xEF` with `(v & 0x0C) != 0x0C`.

## Table — one row per meaningful combination

All rows are differential: both `.so`s are called through their exported `hdr_compare` and
the returned `int`s must be bit-identical. Every row is driven with **many randomized
inputs** (seeded xorshift64\*, fixed seed `0x9E3779B97F4A7C15`) for the axes it does not pin,
plus the exhaustive enumeration where the pinned axes are small.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `hdr_compare` | class **A** sync, layer 1, bitrate `1..14`, samplerate 0, `h1` = exact copy of `h2` (expect match) | `c1_class_a_layer1_norm_sr0_identical` | [x] |
| C2 | `hdr_compare` | class **A** sync, layer 2, bitrate `1..14`, samplerate 1, `h1` = copy | `c2_class_a_layer2_norm_sr1_identical` | [x] |
| C3 | `hdr_compare` | class **A** sync, layer 3, bitrate `1..14`, samplerate 2, `h1` = copy | `c3_class_a_layer3_norm_sr2_identical` | [x] |
| C4 | `hdr_compare` | class **B** sync (`0xE2`), layer 1, bitrate `1..14`, all valid samplerates, `h1` = copy | `c4_class_b_e2_identical` | [x] |
| C5 | `hdr_compare` | class **B** sync (`0xE3`), layer 1, bitrate `1..14`, all valid samplerates, `h1` = copy | `c5_class_b_e3_identical` | [x] |
| C6 | `hdr_compare` | class **A**, bitrate index `0` (free format) on **both** headers ⇒ axis J "both free" | `c6_both_free_format` | [x] |
| C7 | `hdr_compare` | class **A**, bitrate index `1..14` on both ⇒ axis J "both non-free", *different* indices (must still match) | `c7_both_non_free_different_indices` | [x] |
| C8 | `hdr_compare` | class **A**, `h2` free-format, `h1` non-free ⇒ axis J "exactly one free" | `c8_one_free_h2` | [x] |
| C9 | `hdr_compare` | class **A**, `h1` free-format, `h2` non-free ⇒ axis J "exactly one free" (other order) | `c9_one_free_h1` | [x] |
| C10 | `hdr_compare` | axis D: `h1[1]` and `h2[1]` differ **only** in the CRC bit (bit 0) — must be ignored, all 14 valid `h2[1]` × both bit-0 values | `c10_crc_bit_ignored` | [x] |
| C11 | `hdr_compare` | axis G: `h1[2]` and `h2[2]` differ **only** in the padding/private bits `0x03` — must be ignored, all 4×4 combos | `c11_padding_private_bits_ignored` | [x] |
| C12 | `hdr_compare` | axis H: `h1[1]` differs from `h2[1]` in exactly one bit of `1..7` (7 single-bit flips × 14 valid values) | `c12_byte1_single_bit_flips` | [x] |
| C13 | `hdr_compare` | axis I: `h1[2]` differs from `h2[2]` in exactly one bit (8 single-bit flips × all 180 valid `h2[2]`) | `c13_byte2_single_bit_flips` | [x] |
| C14 | `hdr_compare` | axis K: `h1[0]` swept over all 256 values with everything else matching — result must be invariant *and* equal to C's | `c14_h1_byte0_never_read` | [x] |
| C15 | `hdr_compare` | axis A: `h2[0]` swept over all 256 values, rest valid and matching | `c15_h2_byte0_sweep` | [x] |
| C16 | `hdr_compare` | axis L: `h1 == h2` (same pointer, aliased) over all 2^24 `h2` values — result must equal `hdr_valid(h2)` | `c16_aliased_same_pointer_exhaustive` | [x] |
| C17 | `hdr_compare` | axis L: overlapping views — `h1 = buf`, `h2 = buf + 1` and vice-versa, over randomized 8-byte buffers | `c17_overlapping_views` | [x] |
| C18 | `hdr_compare` | axis L: both buffers placed so byte 2 is the **last** readable byte before a `PROT_NONE` page (no over-read allowed) | `c18_page_end_guarded_buffers` | [x] |
| C19 | `hdr_compare` | axis L: unaligned placements — every start offset `0..8` within a 16-byte buffer, randomized contents | `c19_unaligned_offsets` | [x] |
| C20 | `hdr_compare` | full cross-product of the **valid** configuration space: 14 valid `h2[1]` × 180 valid `h2[2]` × all 256 `h1[1]` × all 256 `h1[2]`, `h2[0] = 0xFF` (165 M cases) | `c20_valid_h2_full_cross_product` | [x] |
| C21 | `hdr_compare` | exhaustive over the whole 3-byte `h2` space (2^24) against a battery of fixed `h1` patterns (`00 00 00`, `FF FF FF`, `AA 55 AA`, `55 AA 55`, `FF FB 90`, `FF F3 40`, `FF E3 00`, `FF FF EF`) | `c21_h2_exhaustive_vs_h1_battery` | [x] |
| C22 | `hdr_compare` | **complete** sweep of the reachable input space: `h2[0] = 0xFF` × all 2^32 combinations of `h2[1]`, `h2[2]`, `h1[1]`, `h1[2]` | `c22_full_2p32_sweep` (release, opt-in via `HDR_FULL_SWEEP=1`) | [x] |
| C23 | `hdr_compare` | randomized 3-byte pairs, uniform over all 256 byte values (mostly invalid `h2`) — 4 M samples | `c23_random_uniform` | [x] |
| C24 | `hdr_compare` | randomized pairs biased to **valid** `h2` (sync byte forced `0xFF`, `h2[1]`/`h2[2]` drawn from the valid sets) with fully random `h1` — 4 M samples | `c24_random_valid_h2_random_h1` | [x] |
| C25 | `hdr_compare` | randomized pairs where `h1` is a *mutation* of `h2` (1–3 random bit flips) — the hardest region, both near-match and near-miss — 4 M samples | `c25_random_near_match_mutations` | [x] |
| C26 | `hdr_compare` | randomized pairs with both bytes drawn only from the "interesting" boundary set `{0x00,0x01,0x02,0x03,0x0C,0x0F,0x10,0x7F,0x80,0xE0,0xE2,0xE3,0xEF,0xF0,0xF1,0xFB,0xFE,0xFF}` — 4 M samples | `c26_random_boundary_alphabet` | [x] |
| C27 | `hdr_compare` | layer × bitrate cross-product: all 3 valid layers × all 16 bitrate indices (incl. `0` and `15`) × all 4 samplerate indices × both sync classes, `h1` = copy | `c27_layer_bitrate_samplerate_cross` | [x] |
| C28 | `hdr_compare` | same as C27 but `h1` carries an *independent* layer/bitrate/samplerate triple (full 2-header cross-product of the decoded field tuples) | `c28_two_header_field_tuple_cross` | [x] |
| C29 | `hdr_compare` | real-world MPEG headers: MPEG-1 L3 (`FF FB 90`), MPEG-1 L2 (`FF FD 40`), MPEG-1 L1 (`FF FF 10`), MPEG-2 L3 (`FF F3 40`), MPEG-2.5 L3 (`FF E3 40`), free-format (`FF FB 00`), compared against each other pairwise (all 6×6) | `c29_realworld_header_matrix` | [x] |
| C30 | `hdr_compare` | argument-order asymmetry: for every pair in C29 and in randomized runs, also call `hdr_compare(h2, h1)` — the C is *not* symmetric, so both orders must be checked | `c30_argument_order_asymmetry` | [x] |
| C31 | `hdr_compare` | return-value shape on valid path: result must be exactly `1` when all four conditions hold, exactly `0` otherwise (no other truthy int) | `c31_return_exactly_0_or_1` | [x] |
| C32 | `hdr_compare` | repeated / interleaved calls on the same loaded handles (no hidden state: 1 M alternating C→Rust→C calls must stay in lockstep) | `c32_no_hidden_state_interleaved` | [x] |
| C33 | `hdr_compare` | invariance proof for axis K: `h1[0]` unreadable is impossible, so instead assert result independence over all 256 `h1[0]` values for 4096 randomized remaining-byte configurations | `c33_h1_byte0_invariance_randomized` | [x] |
| C34 | `hdr_compare` | axis A, negative half, exhaustively: `h2[0] ∈ {0x00, 0x01, 0x7F, 0xFE}` × all 2^32 combinations of the other four bytes — every case must be rejected | `c34_non_sync_byte0_full_sweeps` | [x] |
| C35 | `hdr_compare` | axis A, all 256 `h2[0]` values × the full 2^16 `(h2[1], h2[2])` space × the `h1` battery | `c35_all_byte0_values_vs_h1_battery` | [x] |
| C36 | `hdr_compare` | **total** exhaustive equivalence: all 2^40 combinations of every byte the C reads (`h2[0]`, `h2[1]`, `h2[2]`, `h1[1]`, `h1[2]`) | `c36_complete_2p40_sweep` (opt-in, `HDR_SWEEP_2P40=1`; ~12 min) | [x] |

## Feature combinations

`Cargo.toml` declares **no** `[features]` section, so the complete set of cargo feature
combinations is `{default}` ≡ `{--no-default-features}`. `verify.sh` enumerates the power set
of the declared features mechanically (it stays correct if features are ever added) and runs
the whole suite for each combination in **both** the `debug` and `release` profiles.

Under `debug` the heavy sweeps are strided (see `common::stride()`), while the byte range is
unioned with the boundary alphabet and every *valid* header byte so the accepting branches are
still reached; `--release` runs every row at full size. Override with `HDR_STRIDE=1`.
