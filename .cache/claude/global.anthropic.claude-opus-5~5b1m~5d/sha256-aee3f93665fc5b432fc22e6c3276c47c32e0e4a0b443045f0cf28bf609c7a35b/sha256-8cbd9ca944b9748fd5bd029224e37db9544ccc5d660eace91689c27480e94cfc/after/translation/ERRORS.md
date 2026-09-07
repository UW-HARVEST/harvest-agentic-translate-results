# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return -1` statement in the
C source is one row. There are **11** `return -1` statements in `flac_validate`
and **zero** other rejection mechanisms (no `assert`, no `return NULL`, no error
enum, no null checks — `flac_validate` dereferences `t` unconditionally).

`grep -c 'return -1' c_src/src/lib.c` → 11.
`tflac_size_memory` has **no** error path: it is total over all `u32` inputs
(pure wrapping arithmetic), so it contributes no rows.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|----|----------|---------------------------------------------|-------------------|------|--------|
| 1  | `flac_validate` | `t->blocksize < 16` (line 16) — e.g. 0, 1, 15 | `-1`, struct otherwise untouched | `err_01_blocksize_too_small` | [x] |
| 2  | `flac_validate` | `t->blocksize > 65535` (line 18) — e.g. 65536, 0xFFFFFFFF | `-1` | `err_02_blocksize_too_large` | [x] |
| 3  | `flac_validate` | `t->samplerate == 0` (line 20) | `-1` | `err_03_samplerate_zero` | [x] |
| 4  | `flac_validate` | `t->samplerate > 655350` (line 22) — e.g. 655351, 0xFFFFFFFF | `-1` | `err_04_samplerate_too_large` | [x] |
| 5  | `flac_validate` | `t->channels == 0` (line 24) | `-1` | `err_05_channels_zero` | [x] |
| 6  | `flac_validate` | `t->channels > 8` (line 26) — e.g. 9, 0xFFFFFFFF | `-1` | `err_06_channels_too_large` | [x] |
| 7  | `flac_validate` | `t->bitdepth == 0` (line 28) | `-1` | `err_07_bitdepth_zero` | [x] |
| 8  | `flac_validate` | `t->bitdepth > 32` (line 30) — e.g. 33, 0xFFFFFFFF | `-1` | `err_08_bitdepth_too_large` | [x] |
| 9  | `flac_validate` | `t->max_rice_value != 0 && t->max_rice_value > 30` (line 43) — e.g. 31..=255 | `-1`; NOTE the `channel_mode` fix-up at lines 32-36 has ALREADY been applied in-place before this return | `err_09_max_rice_value_too_large` | [x] |
| 10 | `flac_validate` | `t->max_partition_order > 15` (line 46) — e.g. 16..=255 | `-1`; `channel_mode` and `max_rice_value` fix-ups already applied in-place | `err_10_max_partition_order_too_large` | [x] |
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` (line 49) | `-1`; `channel_mode` and `max_rice_value` fix-ups already applied in-place | `err_11_min_gt_max_partition_order` | [x] |

## Ordering / short-circuit semantics (also tested)

The checks are sequential, so when several are violated at once the *first*
failing check decides — but more importantly the **in-place mutations** performed
before a later failure must be identical between C and Rust. Rows 9/10/11 are
therefore asserted on the **whole 28-byte struct image**, not just the return
code.

| # | condition | expected | test | status |
|---|-----------|----------|------|--------|
| O1 | all 11 fields simultaneously invalid | `-1`, struct byte-identical | `err_ordering_all_invalid` | [x] |
| O2 | valid prefix + invalid `max_rice_value` → partial mutation of `channel_mode` | `-1`, struct byte-identical incl. mutated `channel_mode` | `err_partial_mutation_visible` | [x] |

## Generic FFI boundary cases (required even though not in the table)

| # | case | expected | test | status |
|---|------|----------|------|--------|
| G1 | out-of-range `channel_mode` enum values (`4..=255`, i.e. `TFLAC_CHANNEL_MODE_COUNT` and beyond — C enums accept any int) | identical return + identical struct; note C only tests `!= TFLAC_CHANNEL_INDEPENDENT`, so every non-zero value takes the fix-up branch | `err_g1_channel_mode_out_of_range` | [x] |
| G2 | boundary values one step inside/outside every range: blocksize 15/16/65535/65536, samplerate 0/1/655350/655351, channels 0/1/8/9, bitdepth 0/1/16/17/32/33, max_rice_value 0/1/30/31, partition orders 0/15/16 | identical return + identical struct on every combination | `err_g2_boundaries_exhaustive` | [x] |
| G3 | padding bytes (offsets 21..24) filled with garbage — must not be read or written differently | struct byte-identical after the call | `err_g3_padding_preserved` | [x] |
| G4 | `tflac_size_memory` with `0`, `1`, `u32::MAX`, and every value that makes `blocksize * 4` or the `+15` wrap modulo 2^32 | identical `u32` | `size_memory_wrapping` | [x] |
| G5 | `flac_validate` with a null `t` | UB in C (unconditional deref → SIGSEGV). Documented, deliberately NOT called; both sides would crash. | — (documented, not executed) | [x] |

All rows above must be checked before Phase D. See `tests/differential.rs`.
