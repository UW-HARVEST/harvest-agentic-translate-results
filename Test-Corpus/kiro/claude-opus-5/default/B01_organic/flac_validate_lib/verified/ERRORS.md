# ERRORS.md — error-surface table

Mechanically derived from every rejection site in `c_src/src/lib.c`. The file
contains exactly 11 `return -1;` statements plus the implicit UB of a null
`tflac *`; there are no `assert`s, no error enums, and no `return NULL`.

`tflac_size_memory` contains **no** rejection path at all — it is pure wrapping
`u32` arithmetic and every one of the 2^32 inputs is "valid" (covered by
`CONFIGS.md` rows 1–3 instead).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|----|----------|----------------------------------------------|-------------------|------|--------|
| 1  | `flac_validate` | `t->blocksize < 16` (e.g. 0, 1, 15) | `-1`, struct unmodified | `err_01_blocksize_too_small` | [x] |
| 2  | `flac_validate` | `t->blocksize > 65535` (e.g. 65536, 0xFFFFFFFF) | `-1`, struct unmodified | `err_02_blocksize_too_large` | [x] |
| 3  | `flac_validate` | `t->samplerate == 0` | `-1`, struct unmodified | `err_03_samplerate_zero` | [x] |
| 4  | `flac_validate` | `t->samplerate > 655350` (e.g. 655351, 0xFFFFFFFF) | `-1`, struct unmodified | `err_04_samplerate_too_large` | [x] |
| 5  | `flac_validate` | `t->channels == 0` | `-1`, struct unmodified | `err_05_channels_zero` | [x] |
| 6  | `flac_validate` | `t->channels > 8` (e.g. 9, 0xFFFFFFFF) | `-1`, struct unmodified | `err_06_channels_too_large` | [x] |
| 7  | `flac_validate` | `t->bitdepth == 0` | `-1`, struct unmodified | `err_07_bitdepth_zero` | [x] |
| 8  | `flac_validate` | `t->bitdepth > 32` (e.g. 33, 0xFFFFFFFF) | `-1`, struct unmodified | `err_08_bitdepth_too_large` | [x] |
| 9  | `flac_validate` | `t->max_rice_value != 0 && t->max_rice_value > 30` (31..=255) | `-1`, **but `channel_mode` may already have been rewritten to 0** | `err_09_max_rice_value_too_large` | [x] |
| 10 | `flac_validate` | `t->max_partition_order > 15` (16..=255) | `-1`, **but `channel_mode` and `max_rice_value` may already have been mutated** | `err_10_max_partition_order_too_large` | [x] |
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` | `-1`, **but `channel_mode` and `max_rice_value` may already have been mutated** | `err_11_min_gt_max_partition_order` | [x] |
| 12 | `flac_validate` | `t == NULL` — C dereferences it unconditionally (`t->blocksize`), so this is UB that faults | both processes die on `SIGSEGV` (signal 11) | `err_12_null_pointer_same_signal` (subprocess) | [x] |

## Notes on partial mutation (rows 9–11)

Ordering in the C source matters: the `channel_mode` normalisation and the
`max_rice_value` defaulting happen **before** the `max_partition_order` /
`min_partition_order` checks. So a call that ultimately returns `-1` can still
have written to `t->channel_mode` and `t->max_rice_value`. The error-path tests
therefore compare **all 28 struct bytes**, not just the return code.

Rows 9–11 are each exercised with the mutating prefix active
(`channel_mode != 0` with `channels != 2`, and `max_rice_value == 0`) so the
partial-mutation behaviour is actually observed and not accidentally masked.

## Generic FFI boundary cases (also covered)

| case | covered by |
|------|------------|
| null `tflac *` | row 12 |
| zero-valued fields (`samplerate`, `channels`, `bitdepth`, `blocksize`) | rows 1, 3, 5, 7 |
| oversized / `u32::MAX` fields | rows 2, 4, 6, 8 |
| one step past a valid range (16→15, 65535→65536, 655350→655351, 8→9, 32→33, 30→31, 15→16) | rows 1, 2, 4, 6, 8, 9, 10 |
| out-of-range enum value for `channel_mode` (`TFLAC_CHANNEL_MODE_COUNT` == 4 and every value up to 255) | `CONFIGS.md` rows 8–10 + `enum_channel_mode_exhaustive` |
| `partition_order` / `cur_blocksize` pre-seeded with garbage (are they overwritten identically?) | all rows — full-struct byte compare with randomised initial bytes |
