# ERRORS.md — Phase A: error-surface table

Mechanically derived by grepping `c_src/src/lib.c` for **every** rejection:
every `return -1`, every explicit range/equality check, every implicit
constraint. There are no `assert`s, no error enums, no `return NULL`, and no
`RETURN_ERROR`-style macros in this library — the only rejection mechanism is
`return -1` from `flac_validate`. `grep -c 'return -1' c_src/src/lib.c` = 11,
and the table below has exactly 11 `return -1` rows (rows 1–11).

`tflac_size_memory` contains **no** checks and **no** error path: it is a pure
arithmetic expression over `tflac_u32`, total on all 2^32 inputs. It therefore
contributes no rows (its full input domain is covered by `CONFIGS.md`).

| #  | function | trigger (exact invalid input/condition) | expected C result | test | status |
|----|----------|------------------------------------------|-------------------|------|--------|
| 1  | `flac_validate` | `t->blocksize < 16` (e.g. 0, 1, 15) | returns `-1`; struct unmodified | `err_01_blocksize_too_small` | [x] |
| 2  | `flac_validate` | `t->blocksize > 65535` (e.g. 65536, 0xFFFFFFFF) | returns `-1`; struct unmodified | `err_02_blocksize_too_large` | [x] |
| 3  | `flac_validate` | `t->samplerate == 0` | returns `-1`; struct unmodified | `err_03_samplerate_zero` | [x] |
| 4  | `flac_validate` | `t->samplerate > 655350` (e.g. 655351, 0xFFFFFFFF) | returns `-1`; struct unmodified | `err_04_samplerate_too_large` | [x] |
| 5  | `flac_validate` | `t->channels == 0` | returns `-1`; struct unmodified | `err_05_channels_zero` | [x] |
| 6  | `flac_validate` | `t->channels > 8` (e.g. 9, 0xFFFFFFFF) | returns `-1`; struct unmodified | `err_06_channels_too_large` | [x] |
| 7  | `flac_validate` | `t->bitdepth == 0` | returns `-1`; struct unmodified | `err_07_bitdepth_zero` | [x] |
| 8  | `flac_validate` | `t->bitdepth > 32` (e.g. 33, 0xFFFFFFFF) | returns `-1`; struct unmodified | `err_08_bitdepth_too_large` | [x] |
| 9  | `flac_validate` | `t->max_rice_value != 0 && t->max_rice_value > 30` (31..255) — note the `else if`: this branch is only reachable when `max_rice_value != 0`, and it fires **after** `channel_mode` may already have been rewritten to 0 in place | returns `-1`; `channel_mode` may already be mutated to `TFLAC_CHANNEL_INDEPENDENT` | `err_09_max_rice_value_too_large`, `err_09b_max_rice_partial_mutation` | [x] |
| 10 | `flac_validate` | `t->max_partition_order > 15` (16..255) — fires after `channel_mode` and `max_rice_value` may already have been mutated in place | returns `-1`; `channel_mode`/`max_rice_value` may already be mutated | `err_10_max_partition_order_too_large`, `err_10b_partial_mutation` | [x] |
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` (with `max_partition_order <= 15`) | returns `-1`; `channel_mode`/`max_rice_value` may already be mutated; `partition_order`/`cur_blocksize` **not** written | `err_11_min_gt_max_partition_order`, `err_11b_partial_mutation` | [x] |

## Ordering / precedence rows (the checks are sequential — first failure wins)

| #  | function | trigger | expected C result | test | status |
|----|----------|---------|-------------------|------|--------|
| 12 | `flac_validate` | *all* fields simultaneously invalid | `-1` from the **first** check (blocksize) → struct completely unmodified | `err_12_first_check_wins` | [x] |
| 13 | `flac_validate` | every single-field-invalid variant crossed with randomized values in all other fields (2000 randomized cases) | identical return code **and** identical 28 struct bytes | `err_13_randomized_invalid_structs` | [x] |

## Generic FFI boundary rows (required even though not in the C source)

| #  | function | trigger | expected C result | test | status |
|----|----------|---------|-------------------|------|--------|
| 14 | `flac_validate` | `t == NULL` — the C dereferences `t` unconditionally with **no** null check, so this is a hard fault, not a rejection. Verified differentially in a forked child process. | child terminated by `SIGSEGV` (11) in **both** C and Rust | `err_14_null_pointer_faults_identically` (fork-based) | [x] |
| 15 | `flac_validate` | out-of-range "enum" value in `channel_mode`: the field is `tflac_u8`, so 4..255 have no valid `TFLAC_CHANNEL_MODE` variant; C only tests `!= TFLAC_CHANNEL_INDEPENDENT`, so they behave as non-independent modes and are either preserved verbatim or reset to 0 | same return + same `channel_mode` byte for all 256 values × both `channels==2`/`!=2` × `bitdepth==32`/`<32` | `err_15_channel_mode_all_256_values` | [x] |
| 16 | `flac_validate` | one step past each valid range on both sides: blocksize 15/16/65535/65536; samplerate 0/1/655350/655351; channels 0/1/8/9; bitdepth 0/1/32/33; max_rice_value 0/1/30/31; max_partition_order 15/16; min_partition_order == max / max+1 | identical return + identical struct bytes on every side of every boundary | `err_16_one_past_every_boundary` | [x] |
| 17 | `flac_validate` | zero-valued struct (all 28 bytes zero) and all-`0xFF` struct (every field saturated) | `-1` in both (blocksize 0 / blocksize 0xFFFFFFFF) | `err_17_all_zero_and_all_ones` | [x] |
| 18 | `flac_validate` | struct passed with **unaligned-tail garbage**: padding bytes 21..23 pre-filled with `0xAA` on an error path and on the success path | padding preserved identically; no field spill | `err_18_padding_bytes_preserved` | [x] |
| 19 | `tflac_size_memory` | degenerate/oversized inputs: 0, 1, 15, 16, `0x3FFFFFFC` (first `blocksize*4` overflow), `0xFFFFFFFF` — wrapping `unsigned int` arithmetic, never a trap | identical `tflac_u32` (mod 2^32) | `err_19_size_memory_extremes` (+ exhaustive-stride sweep in Phase B) | [x] |
| 20 | `flac_validate` | called **twice** on the same struct (second call observes the first call's in-place mutations, e.g. `max_rice_value` now non-zero, `channel_mode` now 0) | identical return + identical struct bytes after both calls | `err_20_double_call_sees_mutations` | [x] |

All 20 rows must be checked `[x]` before Phase D.

## Phase C result

All 20 rows pass (`tests/phase_c_errors.rs`, 23 tests) under every entry of the
matrix `{default, --no-default-features} x {rust debug, rust release} x {CMake C build, -O2 C build}`.

Every row asserts the same *sentinel* (`-1` vs `0`), not merely "both failed",
and additionally compares all 28 struct bytes so that the partial in-place
mutations a rejected call leaves behind (`channel_mode`, `max_rice_value`) are
compared too.

Row 14 (NULL) initially FAILED and exposed a real divergence: the Rust `.so`
aborted with `SIGABRT` (rustc's debug-assertion "null pointer dereference
occurred", triggered by `&mut *t`) where the C faults with `SIGSEGV`. The Rust
was changed to touch fields only through the raw pointer, and now both die with
`SIGSEGV` in debug and release. See the divergence table in `SYMBOLS.md`.
