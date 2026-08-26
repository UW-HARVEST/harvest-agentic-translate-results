# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, compile definitions, conditional sources, or preprocessor variants.
There is exactly one valid feature combination:

| # | default features | named features | check command |
|---|------------------|----------------|---------------|
| 1 | disabled | none | `cargo check --no-default-features` |

The equivalent test command is `cargo test --no-default-features`.

## Runtime and Input Configurations

This table is derived from all three exported C definitions, including the two
low-level functions not declared in the public header. Randomized value
domains include zero, one, maximum values, sign/high-byte variants, and fixed
seed pseudorandom values. For `update_md5`, every sample case supplies readable
elements through index 135 because those are the fixed indices used by C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tflac_pack_u64le` | Eight-byte destination; randomized `uint64_t` values including `0`, `1`, and `UINT64_MAX` | [x] |
| 2 | `tflac_md5_addsample` | `bits < 8`, so `bytes == 0`; `pos % 64 <= 56`; `new_pos < 64` | [x] |
| 3 | `tflac_md5_addsample` | `bits / 8 > 0` with both exact and fractional bytes; `pos % 64 <= 56`; `new_pos < 64` | [x] |
| 4 | `tflac_md5_addsample` | Packed write begins at `pos % 64` in `57..63`, crossing into the eight-byte staging tail; `new_pos < 64` via zero-byte or unsigned-wrap cases | [x] |
| 5 | `tflac_md5_addsample` | `new_pos >= 64` and `new_pos % 64 == 0`; rollover branch runs with zero copy-loop iterations | [x] |
| 6 | `tflac_md5_addsample` | `new_pos >= 64` and `new_pos % 64 == 1`; rollover branch runs with one copy-loop iteration | [x] |
| 7 | `tflac_md5_addsample` | `new_pos >= 64` and `new_pos % 64` in `2..7`; rollover branch runs with multiple reverse copy-loop iterations within the eight-byte staging tail | [x] |
| 8 | `tflac_md5_addsample` | `pos + bits/8` wraps `uint32_t` to a value below 64; rollover comparison is false after wrapping | [x] |
| 9 | `tflac_md5_addsample` | `total + bits` wraps `uint64_t`; `bits` spans `1..64` and initial `pos` spans `0..63` | [x] |
| 10 | `update_md5` -> low-level functions | Initial MD5 `pos` in `0..23`, so all five additions avoid rollover; `cur_blocksize * channels >= 40` without multiplication wrap | [x] |
| 11 | `update_md5` -> low-level functions | Initial MD5 `pos` in `0..23`, no rollover; product below 40 so the five subtractions wrap `uint32_t` | [x] |
| 12 | `update_md5` -> low-level functions | Initial MD5 `pos` in `0..23`, no rollover; `cur_blocksize * channels` itself wraps `uint32_t` | [x] |
| 13 | `update_md5` -> low-level functions | Initial MD5 `pos` in `24..63`, causing one of the five additions to roll over; non-wrapping product at least 40 | [x] |
| 14 | `update_md5` -> low-level functions | Initial MD5 `pos` in `24..63`, one rollover; product below 40 and return subtraction wraps | [x] |
| 15 | `update_md5` -> low-level functions | Initial MD5 `pos` in `24..63`, one rollover; multiplication wraps `uint32_t` | [x] |
| 16 | `update_md5` -> low-level functions | Initial MD5 `pos >= 64` with `pos % 64` in `56..63`, so the first addition safely normalizes it through the eight-byte staging tail; non-wrapping product at least 40 | [x] |
| 17 | `update_md5` -> low-level functions | Initial MD5 `pos >= 64` with `pos % 64` in `56..63`, first-call normalization; product below 40 and return subtraction wraps | [x] |
| 18 | `update_md5` -> low-level functions | Initial MD5 `pos >= 64` with `pos % 64` in `56..63`, first-call normalization; multiplication wraps `uint32_t` | [x] |

`update_md5` sample generation independently varies low bytes, negative signed
values, and differing upper 24 bits. This covers the C mask to `0xFF`, the
fixed groups at indices `0..7`, `32..39`, `64..71`, `96..103`, and `128..135`,
and the ignored gaps between those groups.
