# CONFIGS.md — Phase A/B: configuration-surface table

## Build-time configurations

`Cargo.toml` has **no `[features]` table** and `src/` contains **no
`#[cfg(feature = ...)]`** (`grep -rn feature src/` -> no matches).
`c_src/CMakeLists.txt` has no `option()`, no `target_compile_definitions`, and
no `#ifdef` in `c_src/src/lib.c` or `c_src/include/lib.h`.

Therefore the **complete** set of valid feature combinations is exactly one: the
empty set. It is verified under both of the ways it can be spelled, plus both
Cargo profiles (`dev` and `release`, which differ here because
`[profile.release] panic = "abort"`):

| # | build configuration | command | symbols | tests | [x] |
|---|---------------------|---------|---------|-------|-----|
| B1 | default (= no features) | `cargo test` | 0 missing | 63 pass | [x] |
| B2 | explicit no-default-features | `cargo test --no-default-features` | 0 missing | 63 pass | [x] |
| B3 | all-features (= no features) | `cargo test --all-features` | 0 missing | 63 pass | [x] |
| B4 | release (`panic = "abort"`) | `cargo test --release` | 0 missing | 63 pass | [x] |
| B5 | release + no-default-features | `cargo test --release --no-default-features` | 0 missing | 63 pass | [x] |

Run all five, with a C-vs-Rust `nm -D` parity check per configuration, via
`./run_all_configs.sh`.

The dev/release split matters here beyond `panic = "abort"`: `debug_assertions`
enables the standard library's UB precondition checks, and optimisation lets LLVM
delete provably-UB accesses. Both changed the observable faulting behaviour of an
earlier version of this translation (see the bug note in `ERRORS.md`), so neither
profile alone would have been sufficient.

### C-side build configurations

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the ground-truth `.so` is
built with **no** optimisation flags. Because the `double`→`unsigned long long`
conversion this library depends on is lowered by the compiler (and is UB for most
inputs), the whole suite was additionally replayed against C rebuilt at `-O0`,
`-O1`, `-O2`, `-O3` and `-Os` — all pass. Point the harness at an alternate C
build with the `IMA_C_SO` environment variable:

```sh
gcc -shared -fPIC -O2 -o /tmp/libO2.so c_src/src/lib.c -Ic_src/include -Ic_src/src
IMA_C_SO=/tmp/libO2.so cargo test
```

## Runtime configuration axes

`ima_parse(struct ima_info *info, const void *data)` is the only public entry
point (`c_src/include/lib.h`), and it takes **no** flags, modes, or length. All
configuration is therefore the *shape and content of the `data` byte stream*.
The axes below are the ones the C actually branches on, taken from its `if` /
`else if` / `for(;;)` structure:

* **A1 — `header->type`** (bytes 0..4): `'caff'` vs. anything else -> `-1`.
* **A2 — `header->version`** (BE u16 at 4..6): `1` vs. anything else -> `-2`.
* **A3 — `header->flags`** (bytes 6..8): never read; must be inert.
* **A4 — chunk list shape**: which of `desc` / `pakt` / `data` appear, in what
  order, how many times, and how many unknown chunks are interleaved.
  `desc` and `pakt` are *latched* (last one before `data` wins) and only `data`
  terminates the loop.
* **A5 — `chunk->size`** (BE s64 at chunk+8): drives the walk stride
  (`chunk += 16 + size`) and, for the `data` chunk, becomes `info->size`.
  Sign, zero, and overflow all matter.
* **A6 — chunk `type` padding** (bytes chunk+4..chunk+8): the 4 bytes of C
  struct tail padding; never read; must be inert.
* **A7 — `desc->format_id`** (chunk+16+8): `'ima4'` vs. anything else -> `-3`.
* **A8 — `desc->sample_rate`** (chunk+16+0): 8 raw bytes read as a *native*
  `double`, then `double`->`u64` value-converted, byte-swapped, and bit-cast.
  The classes the x86-64 lowering distinguishes are the sub-axes below.
* **A9 — `desc->channels_per_frame`** (chunk+16+24): copied out via `bswap32`.
* **A10 — `pakt->frame_count`** (chunk+16+8): copied out via `bswap64`.
* **A11 — ignored `desc`/`pakt` fields**: `format_flags`, `bytes_per_packet`,
  `frames_per_packet`, `bits_per_channel`, `packet_count`, `priming_frames`,
  `remainder_frames`, and `caf_data::edit_count`; all must be inert.
* **A12 — buffer base alignment**: the C casts raw pointers to `struct` types
  that nominally need 8-byte alignment and then loads through them, which x86
  tolerates. The Rust loads through an `align == 1` wrapper, so all 8 residues of
  the buffer base mod 8 must behave identically.

`info` output is compared as all **40 raw bytes** of `struct ima_info` (including
the 4 tail padding bytes after `channel_count`, which neither side may write),
with `blocks` compared as an absolute pointer — both libraries are handed the
*same* buffer, so the pointer value itself must match exactly.

## Configuration rows

Every row is driven with many randomized inputs from a fixed-seed SplitMix64
PRNG (`tests/common/mod.rs`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `ima_parse` | A1 valid, A2 valid, `desc`+`pakt`+`data` in that order, all fields random, `size` random small non-negative | `row01_desc_pakt_data_random_fields` | [x] |
| 2 | `ima_parse` | A4: `pakt` before `desc` (reversed latch order) | `row02_pakt_before_desc` | [x] |
| 3 | `ima_parse` | A4: `data` chunk first-and-only after a `desc`+`pakt`; zero unknown chunks | `row03_no_filler_chunks` | [x] |
| 4 | `ima_parse` | A4: 1 unknown chunk interleaved between `desc` and `pakt` | `row04_one_filler_between_desc_and_pakt` | [x] |
| 5 | `ima_parse` | A4: many (8..32) unknown chunks with random types/sizes interleaved | `row05_many_filler_chunks` | [x] |
| 6 | `ima_parse` | A4: duplicate `desc` chunks — the **last** before `data` must win | `row06_duplicate_desc_last_wins` | [x] |
| 7 | `ima_parse` | A4: duplicate `pakt` chunks — the **last** before `data` must win | `row07_duplicate_pakt_last_wins` | [x] |
| 8 | `ima_parse` | A4: `desc`/`pakt` appearing *after* the `data` chunk (must be ignored — loop already broke) | `row08_chunks_after_data_ignored` | [x] |
| 9 | `ima_parse` | A5: `data` chunk `size` = 0 | `row09_data_size_zero` | [x] |
| 10 | `ima_parse` | A5: `data` chunk `size` = random full-range `i64` (incl. negative), -> `info->size` | `row10_data_size_random_full_range` | [x] |
| 11 | `ima_parse` | A5: `data` chunk `size` = 15-value boundary set {0, ±1, ±2, i64::MIN, i64::MIN+1, i64::MAX, i64::MAX-1, ±2^31, 2^32, ±2^47, -16} | `row11_data_size_boundaries` | [x] |
| 12 | `ima_parse` | A5: unknown chunk with `size` = 0 (stride exactly 16) | `row12_filler_chunk_size_zero` | [x] |
| 13 | `ima_parse` | A5: unknown chunk with negative `size` that walks backwards onto a valid `data` chunk | `row13_backward_walk_onto_data_chunk` | [x] |
| 14 | `ima_parse` | A6: chunk tail-padding bytes randomized on every chunk (must be inert) | `row14_chunk_padding_inert` | [x] |
| 15 | `ima_parse` | A3: `header->flags` swept over random + boundary values (must be inert) | `row15_header_flags_inert` | [x] |
| 16 | `ima_parse` | A8: `sample_rate` = realistic big-endian `f64` (44100/48000/8000/96000) — the common real-world case, which reads as a tiny subnormal and truncates to 0 | `row16_realistic_big_endian_sample_rates` | [x] |
| 17 | `ima_parse` | A8: `sample_rate` raw bytes = uniformly random 8 bytes (hits negative/NaN/Inf/huge -> the UB region) | `row17_sample_rate_uniform_random_bytes` | [x] |
| 18 | `ima_parse` | A8: `sample_rate` LE bit pattern chosen so the native `double` is in `[0, 2^63)` — the well-defined conversion range | `row18_sample_rate_in_well_defined_range` | [x] |
| 19 | `ima_parse` | A8: `sample_rate` native `double` in `[2^63, 2^64)` — the `subsd`/`xor` branch | `row19_sample_rate_in_subsd_branch` | [x] |
| 20 | `ima_parse` | A8: `sample_rate` native `double` `>= 2^64` and `= +Inf` — indefinite via the `subsd` branch | `row20_sample_rate_at_or_above_two_pow_64` | [x] |
| 21 | `ima_parse` | A8: `sample_rate` native `double` negative (incl. `-0.0`, `-Inf`, small negatives, `< -2^63`) | `row21_sample_rate_negative` | [x] |
| 22 | `ima_parse` | A8: `sample_rate` native `double` = quiet NaN, signalling NaN, and NaN with random payloads | `row22_sample_rate_nan` | [x] |
| 23 | `ima_parse` | A8: `sample_rate` native `double` at exact conversion boundaries: `2^63-1024`, `2^63`, `2^63+2048`, `2^64-2048`, `2^64`, `-2^63`, `-2^63-2048`, `±f64::MIN_POSITIVE`, `±0.0`, subnormals | `row23_sample_rate_exact_conversion_boundaries` | [x] |
| 24 | `ima_parse` | A9: `channels_per_frame` = random u32 + boundaries {0,1,2,0x7fffffff,0x80000000,0xffffffff} | `row24_channel_count_values` | [x] |
| 25 | `ima_parse` | A10: `frame_count` = random u64 + boundaries {0,1,-1,i64::MIN,i64::MAX,u64::MAX} | `row25_frame_count_values` | [x] |
| 26 | `ima_parse` | A11: every ignored field set to random garbage (must be inert) | `row26_ignored_fields_inert` | [x] |
| 27 | `ima_parse` | A12: buffer base offset by 0..8 bytes within a larger allocation (misaligned `desc`/`pakt`/`chunk`) | `row27_misaligned_buffer_base` | [x] |
| 28 | `ima_parse` | A7 valid + full random cross-product: random chunk-list shape x random `size` x random `sample_rate` bytes x random ignored fields, **200 000** iterations | `row28_full_random_cross_product` | [x] |
| 29 | `ima_parse` | `blocks` output pointer: asserted to equal `buf + (data chunk offset) + 20` for random chunk-list shapes | `row29_blocks_pointer_offset` | [x] |
| 30 | `ima_parse` | A2: all 65536 `header->version` values swept (exactly one must yield != -2) | `row30_sweep_all_65536_version_values` | [x] |
| 31 | `ima_parse` | A1: random 4-byte `header->type` values incl. every 1-byte mutation of `"caff"` | `row31_header_type_values_and_mutations` | [x] |
| 32 | `ima_parse` | A16/row-16: random 4-byte chunk types incl. every 1-byte mutation of `"desc"`/`"pakt"`/`"data"` | `row32_chunk_type_values_and_mutations` | [x] |
| 33 | `ima_parse` | `info` pre-filled with a poison pattern; all 40 bytes compared, incl. tail padding, on both success and every error return | `row33_full_info_struct_including_padding` | [x] |
| 34 | `ima_parse` | A8 saturation fuzz: 10^6 `sample_rate` bit patterns (uniform random, plus exponent ranges targeted at the `2^63`/`2^64` edges, plus NaN payloads), each cross-checked against both the C `.so` and an independent model of the x86-64 lowering. Asserts measured coverage of all four conversion branches. | `fuzz_sample_rate_pipeline` | [x] |

Row counts per test are set by `ITERS` (20 000) in `tests/phase_b_valid.rs`,
except where a row states otherwise or sweeps exhaustively (rows 30, 31, 32).

## Coverage actually achieved on the risky path

`fuzz_sample_rate_pipeline` reports the branch distribution it reached, so the
fuzz cannot silently degenerate into one easy path:

| conversion branch | inputs |
|---|---|
| native `double` in `[0, 2^63)` (direct `cvttsd2si`) | 371 611 |
| native `double` >= `2^63` (`subsd` + `xor` path) | 191 541 |
| native `double` negative | 311 709 |
| native `double` NaN | 125 139 |

## Entry points

`ima_parse` is the only symbol in the library's dynamic table, so it is both the
highest- and the lowest-level public entry point; there is no convenience wrapper
to mistake for the real API. The `static` helpers (`ima_bswap16/32/64`,
`ima_btoh16/32/64`) are not reachable by an external caller, and are covered
through exact output oracles instead — see `SYMBOLS.md`.
