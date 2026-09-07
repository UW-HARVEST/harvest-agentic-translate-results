# CONFIGS.md — Phase B configuration surface table

## Axes, derived from the branches the C actually takes

`c_src/src/lib.c` has no runtime option struct, no `#ifdef`, no global mode
flags. Every branch is driven by the four call arguments and the two structs.
The complete list of conditions the C source branches on:

| axis | source line | distinct states the C treats differently |
|------|-------------|-------------------------------------------|
| A. `sci->total_bands` | `i < 2 * sci->total_bands` (l.22) | `0` (loop skipped) · `1` (2 bands) · small (`2..16`) · `32` (`i` exactly fills `bitalloc[64]`) · `33..64` (`i` runs into `scfcod[]`, still inside the object) |
| B. `sci->bitalloc[i]` | `if (ba != 0)` (l.24), `if (ba < 17)` (l.25) | `0` (skip) · `1` (`half = 0`) · `2..15` (linear path) · `16` (`half = 0x7FFF`, max linear) · `17..24` (grouped, `mod = 3..257`, readable widths) · `25..46` (grouped, `n` huge) · `47` (`mod = 0x80000001`) · `48` (`mod = 1`) · `49..255` (`(ba-17)&31` wrap) · **mixed** values across bands |
| C. `group_size` (k loop / `dst` base stride) | `dst = grbuf + group_size*j` (l.21), `k < group_size` (l.27/33) | `0` · negative · `1` · `2` · `3` · `4` · `12` · `18` (real MP3 value) · large |
| D. `bs->pos & 7` (bit phase `s`) | `s = bs->pos & 7` (l.4), `255 >> s` (l.9) | all 8 phases `0..7` — each gives a different first-byte mask and a different `shl` byte count |
| E. `bs->limit` vs demand | `(bs->pos += n) > bs->limit` (l.7) | ample (never hit) · exactly equal at a field end (accepted) · one bit short (rejected) · crossed mid-granule · `0` · negative |
| F. `bs->buf` contents | `*p++` (l.9, l.12) | randomized (many seeds) · all `0x00` · all `0xFF` · alternating patterns |
| G. `choff` phase | `choff = 18 - choff` (l.39), **initialised once outside the `j` loop** (l.19) | the band count `2*total_bands` is always **even**, so the +576/−558 toggle is applied an even number of times per granule and `choff` is provably back at 576 at every granule boundary. The `dst` *walk* (0, +576, +18, +594, +36, …) still has to be reproduced step for step, and the `j`-carry of `dst = grbuf + group_size*j` differs per granule. (Verified as an equivalence: `scripts/mutation_check.sh` shows that re-initialising `choff` per granule is an *undetectable* mutant, while perturbing the 576 or the 18 is caught.) |
| H. field-width/byte-span in `get_bits` | `while ((shl -= 8) > 0)` (l.10) | `n + s <= 8` (single byte, loop body never runs) · `9..16` (1 iteration) · `17..24` · `>= 25` (many iterations) |
| I. sign of `bs->pos` **at a real read** | `bs->buf + (bs->pos >> 3)` (l.6) | `pos >= 0` · `pos < 0` **with the read actually taken** — only observable when `bs->buf` points into the middle of a larger allocation, otherwise the arithmetic-vs-logical `>> 3` is invisible |
| J. `total_bands` past the object | `sci->bitalloc[i]`, `i < 2*total_bands` (l.22–23) | `<= 64` (inside the struct) · `65..127` · `128..255` (walks up to `i == 509`) — made comparable by embedding `L12_scale_info` in a padded allocation with test-controlled trailing bytes |

## Public entry points

There is exactly one exported entry point, `dequantize_granule`. The
lowest-level function, `get_bits`, is `static` and is driven **only** through
`dequantize_granule` — so every row below drives the composed pipeline
end-to-end (bitstream state → band loop → `choff` walk → float writes), which is
the only way to reach `get_bits`' own axes (D, E, H).

Observables compared for every row: the full `grbuf` (bit-exact `f32` patterns
over the whole allocation, so stray/out-of-range writes are caught),
`bs->pos`, `bs->limit`, the entire `L12_scale_info` (catches unintended writes),
and the `int` return value.

## Rows

Each row is run with **many randomized inputs** (fixed seed `0x5EED_1234`,
32–64 cases per row unless noted) over the unconstrained axes.

| # | entry point | configuration (options set + input shape) | test | [x] |
|---|-------------|--------------------------------------------|------|-----|
| C1 | `dequantize_granule` | A=0, C=18, E=ample — degenerate: no bands | `c1_no_bands` | [x] |
| C2 | `dequantize_granule` | A=1, B=1 (`half=0`), C=18, D=0, E=ample, F=random | `c2_ba1_minimal` | [x] |
| C3 | `dequantize_granule` | A=1, B∈{2..15} swept one value per case, C=18, D=0, E=ample | `c3_ba_linear_sweep` | [x] |
| C4 | `dequantize_granule` | A=1, B=16 (max linear, `half=0x7FFF`), C=18, D=0, E=ample, H=`n+s∈{16..23}` | `c4_ba16_max_linear` | [x] |
| C5 | `dequantize_granule` | A=1, B∈{1..16}, C=18, **D swept 0..7** (all bit phases), E=ample | `c5_all_bit_phases` | [x] |
| C6 | `dequantize_granule` | A=1, B=1..8 chosen so `n+s <= 8` (single-byte fast path, `while` never entered), D=0..7 | `c6_single_byte_fields` | [x] |
| C7 | `dequantize_granule` | A=1, B=16, D=7 ⇒ `shl=23` (multi-iteration `while`), E=ample | `c7_multi_byte_span` | [x] |
| C8 | `dequantize_granule` | A=1, B=17 (`mod=3`, `n=5`), C=18, D=0..7, E=ample — grouped path | `c8_grouped_ba17` | [x] |
| C9 | `dequantize_granule` | A=1, B∈{18..24} swept (`mod=5,9,17,33,65,129,257`), C=18, D random, E=ample | `c9_grouped_ba_sweep` | [x] |
| C10 | `dequantize_granule` | A=1, B=48 (`mod=1` via `2<<31 == 0`), all outputs `0.0f`, `n=3` | `c10_grouped_mod_one` | [x] |
| C11 | `dequantize_granule` | A=1, B∈{49,50,51,55,64,255} — masked shift `(ba-17)&31` aliasing, E=negative (no reads) | `c11_grouped_shift_alias` | [x] |
| C12 | `dequantize_granule` | A=2..16 random, B=**mixed** random per band incl. zeros, C=18, D random, E=ample, F=random | `c12_mixed_bands_random` | [x] |
| C13 | `dequantize_granule` | A=32 (`i` covers exactly `bitalloc[0..63]`), B=mixed random, C=18, E=ample | `c13_total_bands_32_full` | [x] |
| C14 | `dequantize_granule` | A∈{33..64} (`i` runs into `scfcod[]`, still in-object), B+`scfcod` mixed random, C=18, E=ample | `c14_total_bands_into_scfcod` | [x] |
| C15 | `dequantize_granule` | C swept over `{1,2,3,4,5,6,7,8,12,16,18,24,32}`, A=2, B=mixed, E=ample | `c15_group_size_sweep` | [x] |
| C16 | `dequantize_granule` | C=1 with B>=17 grouped path (`code` divided once), A=4 | `c16_group_size_one_grouped` | [x] |
| C17 | `dequantize_granule` | C=18 (real MP3), A=32, B=mixed 1..16 only, F=all `0x00` | `c17_buf_all_zeros` | [x] |
| C18 | `dequantize_granule` | C=18, A=32, B=mixed 1..16 only, F=all `0xFF` | `c18_buf_all_ones` | [x] |
| C19 | `dequantize_granule` | C=18, A=32, B=mixed, F=`0xAA/0x55` alternating and `0x0F/0xF0` | `c19_buf_patterns` | [x] |
| C20 | `dequantize_granule` | E=exact boundary: `limit` set so the *last* field ends precisely at `limit` | `c20_limit_exact` | [x] |
| C21 | `dequantize_granule` | E=crossed mid-granule (`limit` = 60% of demand), A=16, B=mixed, C=18 | `c21_limit_mid` | [x] |
| C22 | `dequantize_granule` | G: 4-granule `choff` carry — A∈{1,2,3,5}, C∈{1,2,18}, verifies the `dst` walk `+576, −558, +576, …` and its per-granule `group_size*j` base | `c22_choff_carry_across_granules` | [x] |
| C23 | `dequantize_granule` | Full-fuzz cross-product: A,B,C,D,E,F all randomized together, 4000 cases | `c23_full_random_fuzz` | [x] |
| C24 | `dequantize_granule` | `bs->pos` starting at a large non-zero bit offset (mid-buffer), all other axes random | `c24_nonzero_start_pos` | [x] |
| C25 | `dequantize_granule` | I: `bs->buf` pointed 32 KiB into a larger allocation, `bs->pos` **negative** and the read actually taken; every negative bit phase −1..−64, plus 256 randomized cases | `e7b_negative_pos_with_real_reads` | [x] |
| C26 | `dequantize_granule` | I × E × A: same mid-allocation `bs->buf`, positions of **both signs**, limits ample/partial/negative, `total_bands` 0..64, 2000 randomized cases | `e7c_origin_fuzz_mixed_sign_positions` | [x] |
| C27 | `dequantize_granule` | J: `L12_scale_info` embedded in a padded allocation; **every** `total_bands` 0..=255 (band index up to 509, far past the struct), small band widths | `e17b_total_bands_65_to_255_padded` | [x] |
| C28 | `dequantize_granule` | J: `total_bands >= 128` (high bit set) with live band widths **only** past the end of the struct — catches a sign-extending `total_bands` read | `e17c_total_bands_high_bit_set` | [x] |
| C29 | `dequantize_granule` | J × B × C × E: padded-struct fuzz, `total_bands` 0..=255, mixed widths incl. grouped, mixed limits, 1500 cases | `e17d_padded_fuzz` | [x] |

## Result

All 29 rows pass, under both the `debug` and `release` profiles and both
feature configurations (`default` and `--no-default-features`) — see
`scripts/check_all.sh`.

Randomized-case totals actually executed (not merely attempted):

| test | cases run |
|------|-----------|
| `c23_full_random_fuzz` | 4000 / 4000 (0 skipped) |
| `e7c_origin_fuzz_mixed_sign_positions` | 2000 |
| `e17d_padded_fuzz` | 1500 |
| `e7b_negative_pos_with_real_reads` | 256 + 128 phase-sweep |

## Harness adequacy

Passing tests only prove something if the tests *can* fail. `scripts/mutation_check.sh`
injects 51 deliberate defects into `src/lib.rs` (one at a time), rebuilds the
Rust `.so` and re-runs the whole suite:

* **46 mutants are killed** — every off-by-one, wrong shift, wrong branch
  threshold, wrong `choff` constant, wrong `dst` index, signedness error and
  bounds-clamping "safe Rust" fix is detected.
* **5 mutants survive, each with a written equivalence proof** (`choff`
  re-initialisation per granule; `saturating_add`/plain `-=` where overflow is
  unreachable; `>= 0` loop exit whose extra iteration contributes `next >> 8 == 0`;
  two's-complement subtraction done in signed instead of unsigned space).
* **0 unexplained survivors.**
