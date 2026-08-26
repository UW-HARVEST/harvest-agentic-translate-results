# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c`. `grep -n 'return\|assert\|NULL'`
finds **12 return statements** in the file:

* line 12 — `tflac_size_memory` normal return (no error path at all)
* lines 17, 19, 21, 23, 25, 27, 29, 31, 44, 47, 50 — **11 distinct `return -1`
  rejection branches** in `flac_validate`
* line 57 — `flac_validate` success `return 0`

There are **no** `assert()`s, **no** `NULL` checks, **no** error enums and **no**
error-return macros in the library. `flac_validate` dereferences `t`
unconditionally on line 16.

## Important: the struct is mutated *before* some rejections

`flac_validate` writes to `*t` **before** reaching the last three rejection
branches, so an error-path test MUST compare the whole 28-byte `struct tflac`
after the call, not only the `int` return value:

* `t->channel_mode` may be forced to `TFLAC_CHANNEL_INDEPENDENT` (line 34)
  before rows 9, 10 and 11 fire.
* `t->max_rice_value` may be auto-filled with `14`/`30` (lines 39, 41) before
  rows 10 and 11 fire.

Rows 1–8 reject before any write, so the struct must come back byte-identical.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `flac_validate` | `t->blocksize < 16` (line 16) — e.g. `blocksize = 0`, `15`; all other fields valid | returns `-1`; `*t` completely unmodified |
| 2 | `flac_validate` | `t->blocksize > 65535` (line 18) — e.g. `65536`, `0xFFFFFFFF` | returns `-1`; `*t` completely unmodified |
| 3 | `flac_validate` | `t->samplerate == 0` (line 20), `blocksize` in range | returns `-1`; `*t` completely unmodified |
| 4 | `flac_validate` | `t->samplerate > 655350` (line 22) — e.g. `655351`, `0xFFFFFFFF` | returns `-1`; `*t` completely unmodified |
| 5 | `flac_validate` | `t->channels == 0` (line 24) | returns `-1`; `*t` completely unmodified |
| 6 | `flac_validate` | `t->channels > 8` (line 26) — e.g. `9`, `0xFFFFFFFF` | returns `-1`; `*t` completely unmodified |
| 7 | `flac_validate` | `t->bitdepth == 0` (line 28) | returns `-1`; `*t` completely unmodified |
| 8 | `flac_validate` | `t->bitdepth > 32` (line 30) — e.g. `33`, `0xFFFFFFFF` | returns `-1`; `*t` completely unmodified |
| 9 | `flac_validate` | `t->max_rice_value != 0 && t->max_rice_value > 30` (lines 43–44) — e.g. `31`, `255` | returns `-1`; `channel_mode` **may already be zeroed**; `max_rice_value` unchanged; `partition_order`/`cur_blocksize` unchanged |
| 10 | `flac_validate` | `t->max_partition_order > 15` (lines 46–47) — e.g. `16`, `255` | returns `-1`; `channel_mode` may already be zeroed **and** `max_rice_value` may already be auto-filled to `14`/`30`; `partition_order`/`cur_blocksize` unchanged |
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` (lines 49–50), with `max_partition_order <= 15` — e.g. `min=1,max=0`; `min=15,max=14` | returns `-1`; same partial mutations as row 10; `partition_order`/`cur_blocksize` unchanged |

### Rejection-precedence note

The checks are a straight-line sequence, so when **several** triggers hold at
once the *earliest* one wins and determines how much of `*t` was mutated. The
test suite therefore also drives multi-trigger structs (e.g. `blocksize = 0`
**and** `max_rice_value = 255`) and asserts the same `(ret, struct)` pair.

## Generic FFI boundaries (covered even though not in the table above)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| G1 | `flac_validate` | `t == NULL` — the C code has **no** null check and dereferences on line 16 | process dies with `SIGSEGV` (undefined behaviour in C). Rust must behave identically — verified in a forked child process. |
| G2 | `flac_validate` | `channel_mode` out of every declared `enum TFLAC_CHANNEL_MODE` variant: `4` (`TFLAC_CHANNEL_MODE_COUNT`) … `255`. The field is `tflac_u8`, and C compares only `!= TFLAC_CHANNEL_INDEPENDENT`, so **every non-zero value takes the "not independent" branch** | success (`0`) when the rest is valid. `channel_mode` is **left at its out-of-range value** when `channels == 2 && bitdepth != 32`, otherwise forced to `0`. No rejection. |
| G3 | `flac_validate` | `max_rice_value` one step past valid: `30` → ok, `31` → reject | `30` ⇒ `0`, `31` ⇒ `-1` |
| G4 | `flac_validate` | `max_partition_order` one step past valid: `15` → ok, `16` → reject | `15` ⇒ `0`, `16` ⇒ `-1` |
| G5 | `flac_validate` | `blocksize` one step past each bound: `15`/`16` and `65535`/`65536` | `15` ⇒ `-1`, `16` ⇒ `0`, `65535` ⇒ `0`, `65536` ⇒ `-1` |
| G6 | `flac_validate` | `samplerate` one step past each bound: `0`/`1` and `655350`/`655351` | `0` ⇒ `-1`, `1` ⇒ `0`, `655350` ⇒ `0`, `655351` ⇒ `-1` |
| G7 | `flac_validate` | `channels` one step past each bound: `0`/`1` and `8`/`9` | `0` ⇒ `-1`, `1` ⇒ `0`, `8` ⇒ `0`, `9` ⇒ `-1` |
| G8 | `flac_validate` | `bitdepth` one step past each bound: `0`/`1` and `32`/`33` | `0` ⇒ `-1`, `1` ⇒ `0`, `32` ⇒ `0`, `33` ⇒ `-1` |
| G9 | `flac_validate` | `partition_order` / `cur_blocksize` pre-seeded with garbage (they are pure outputs) | on success both are overwritten; on rejection both keep the garbage |
| G10 | `tflac_size_memory` | zero and oversized lengths: `0`, `0x3FFFFFFF`, `0x40000000` (`blocksize * 4U` wraps), `0xFFFFFFFF` | no error path — pure `u32` wrapping arithmetic; must match bit-for-bit |

## Row check-off (Phase C)

| row | test | status |
|-----|------|--------|
| 1 | `err_row01_blocksize_too_small` | [x] |
| 2 | `err_row02_blocksize_too_large` | [x] |
| 3 | `err_row03_samplerate_zero` | [x] |
| 4 | `err_row04_samplerate_too_large` | [x] |
| 5 | `err_row05_channels_zero` | [x] |
| 6 | `err_row06_channels_too_large` | [x] |
| 7 | `err_row07_bitdepth_zero` | [x] |
| 8 | `err_row08_bitdepth_too_large` | [x] |
| 9 | `err_row09_max_rice_value_too_large` | [x] |
| 10 | `err_row10_max_partition_order_too_large` | [x] |
| 11 | `err_row11_min_gt_max_partition_order` | [x] |
| precedence | `err_rejection_precedence_multi_trigger` | [x] |
| G1 | `err_g1_null_pointer_segv_parity` (+ `err_g1_null_child`) | [x] |
| G2 | `err_g2_out_of_range_channel_mode_enum` | [x] |
| G3 | `err_g3_max_rice_value_one_past` | [x] |
| G4 | `err_g4_max_partition_order_one_past` | [x] |
| G5 | `err_g5_blocksize_one_past` | [x] |
| G6 | `err_g6_samplerate_one_past` | [x] |
| G7 | `err_g7_channels_one_past` | [x] |
| G8 | `err_g8_bitdepth_one_past` | [x] |
| G9 | `err_g9_output_fields_are_pure_outputs` | [x] |
| G10 | `err_g10_size_memory_extremes` | [x] |

## Notes on how each error path is asserted

`tests/common/mod.rs::check_validate_ret` compares, between the C `.so` and
both Rust `.so`s:

1. the `int` return value (the exact sentinel `-1` / `0`, not merely
   "both failed"), **and**
2. all **28 bytes** of `struct tflac` after the call, tail padding included.

`tests/diff_errors.rs` adds per-row premise assertions so a row cannot silently
stop testing what it claims:

* rows 1–8 additionally assert the struct is returned **byte-identical** to the
  input (`expect_reject_unmodified`);
* rows 9–11 additionally assert `partition_order` and `cur_blocksize` are
  **not** written, and pin down the partial mutations (`channel_mode` zeroing,
  `max_rice_value` auto-fill) that precede the rejection.

### Exhaustive error-path backstops

| test | space enumerated |
|------|------------------|
| `err_exhaustive_u8_fields` | all 256×256 `(min_partition_order, max_partition_order)` pairs, all 256×256 `(max_rice_value, max_partition_order)` pairs, and all 256×256 `(channel_mode, max_rice_value)` pairs — **196 608** configurations, each with the exact expected verdict pinned |
| `err_exhaustive_u32_field_boundaries` | `blocksize` `0..=70000`, `samplerate` `0..=660000`, `channels` `0..=1024`, `bitdepth` `0..=1024`, plus `u32::MAX`, `u32::MAX-1`, `2^31`, `2^16` for each — every threshold crossing enumerated with the exact expected verdict |

### Harness negative control

`./mutation_check.sh` injects **28 deliberate bugs** into `src/lib.rs` (each
error bound off by one, each rejection dropped, each auto-fill constant
changed, the loop shift/cap/comparison altered, the `size_memory` constants and
wrapping altered, and each `#[unsafe(no_mangle)]` export removed) and asserts
the suite fails for every one. Result: **28 / 28 mutants detected**, so no row
in this table is vacuously passing.
