# CONFIGS.md — Phase B configuration-surface table

## Public entry points (complete)

`c_src/include/lib.h` declares exactly one function:

```c
int ima_parse(struct ima_info *info, const void *data);
```

There are no convenience wrappers and no additional exported functions — the
whole C library is one translation unit whose only external symbol is
`ima_parse` (see `SYMBOLS.md`).  The lower-level routines
(`ima_bswap16/32/64`, `ima_btoh16/32/64`) are `static`, so the only way to reach
them is through `ima_parse`; they are therefore driven *individually and
exhaustively* by feeding each field they convert:

| static routine | driven through | coverage |
|----------------|----------------|----------|
| `ima_btoh16` / `ima_bswap16` | `header->version` | **exhaustive**, all 65 536 values (row 3) |
| `ima_btoh32` / `ima_bswap32` | `header->type`, `chunk->type`, `desc->format_id`, `desc->channels_per_frame` | 4 × randomized 32-bit fuzz (rows 1, 17, 18 and ERRORS rows 12-14) |
| `ima_btoh64` / `ima_bswap64` | `chunk->size`, `pakt->frame_count` | randomized 64-bit fuzz + extremes (rows 13, 15) |
| the `(ima_u64_t)double` value conversion at `lib.c:127` | `desc->sample_rate` | random bit patterns + biased magnitudes + curated hard doubles (rows 19, 19b, 20) |

## Axes the C code actually branches on

Derived from every `if` / `else if` / `for` / conversion in `ima_parse`:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. `header->type` | `== 'ffac'` (bytes `"caff"`) / `!=` | `lib.c:87` |
| B. `header->version` | `== 1` / `!=` | `lib.c:92` |
| C. `header->flags` | **never read** (bytes 6..8 are don't-care) | absence of any read |
| D. `chunk->type` | `'csed'`(`"desc"`) / `'tkap'`(`"pakt"`) / `'atad'`(`"data"`) / any of the other 2^32−3 values | `lib.c:97,102,107` |
| E. chunk ordering | `desc` before/after `data`; `pakt` before/after `data`; `desc` before/after `pakt` | control flow of `for(;;)` |
| F. chunk multiplicity | 0 / 1 / many `desc`, 0 / 1 / many `pakt` (**the last one before `data` wins**); 1 / many `data` (**the first one wins**, `break`) | overwriting assignment + `break` |
| G. unknown chunks skipped | 0 / 1 / many | loop iteration count |
| H. `chunk->size` | `0`, positive, **negative (the scan walks backwards)**, `-16` (self-loop), `i64::MIN`, `i64::MAX` | `lib.c:115` pointer arithmetic |
| I. chunk stride | **`16 + size`**, because `sizeof(struct caf_chunk) == 16` (4 bytes of padding between `type` and `size`) — *not* the 12 bytes real CAF uses.  Bytes 4..8 of every chunk are don't-care, and a declared size smaller than the payload makes the next chunk header **overlap** the previous payload. | `sizeof(struct caf_chunk)` |
| J. `desc->format_id` | `== '4ami'` (bytes `"ima4"`) / `!=` | `lib.c:118` |
| K. `desc->sample_rate` (`double`) | the `(ima_u64_t)double` **value** conversion has three hardware paths: `x >= 2^63` (bias + `xor 2^63`), ordered `x < 2^63` (plain `cvttsd2si`), and unordered / out-of-range (`0x8000000000000000` "integer indefinite").  Sub-cases: `+0.0`, `-0.0`, small positive, small negative, fractional (truncation toward zero), subnormal, exactly `2^63`, `> 2^63`, `< -2^63`, `+inf`, `-inf`, qNaN, sNaN, arbitrary bit patterns. | `lib.c:127` (`comisd`/`jae`/`subsd`/`cvttsd2si`/`xor`) |
| L. `desc->channels_per_frame` | any `u32` | `lib.c:126` |
| M. `pakt->frame_count` | any `u64` | `lib.c:125` |
| N. `data` pointer alignment | offsets 0..7 — the C casts the buffer to `struct caf_*` with no alignment guarantee | `lib.c:76` |
| O. unread fields | `desc->format_flags / bytes_per_packet / frames_per_packet / bits_per_channel`, `pakt->packet_count / priming_frames / remainder_frames`, `caf_data->edit_count`, `ima_block` contents | absence of reads |
| P. `info->blocks` output | must equal `&data_chunk + 16 + 4` exactly | `lib.c:111` |
| Q. `info->size` output | the **`data`** chunk's `chunk_size`, `s64` → `u64` reinterpreted (*not* the last chunk scanned) | `lib.c:124` |

Axes C, I(padding) and O are "don't-care" axes: **every** row below fills those
bytes with seeded pseudo-random noise, so any accidental Rust read of them
diverges.  Row 14 additionally proves it by re-running the same semantic file
with a different noise filling.

## Configuration rows

Every row drives **both** `.so`s via `libloading` with the **same** input
pointer and compares the `int` return value *and* all 40 bytes of
`struct ima_info` (tail padding included; `sample_rate` compared as raw bits so
that NaNs are distinguished).  Every row uses many seeded randomized inputs, and
each row additionally asserts the C return value it is supposed to be exercising
so that it can never pass vacuously.

Tests live in `tests/phase_b_valid.rs`.

| #  | entry point | configuration (options set + input shape) | iters | test | [x] |
|----|-------------|-------------------------------------------|-------|------|-----|
| 0  | — | both `.so`s really are two distinct files with two distinct `ima_parse` addresses | 1 | `cfg00_libraries_are_distinct_shared_objects` | [x] |
| 1  | `ima_parse` | A✗: random non-`"caff"` magic, random version, random trailing bytes | 20 000 | `cfg01_bad_magic_randomized` | [x] |
| 2  | `ima_parse` | A✓ + B✗: valid magic, random `version != 1` | 20 000 | `cfg02_bad_version_randomized` | [x] |
| 3  | `ima_parse` | A✓ + B: **exhaustive** over all 65 536 `version` values behind an otherwise valid file (only `1` proceeds) | 65 536 | `cfg03_version_exhaustive` | [x] |
| 4  | `ima_parse` | minimal valid: `desc`(size 32), `pakt`(size 24), `data`; random K/L/M/Q | 5 000 | `cfg04_minimal_valid` | [x] |
| 5  | `ima_parse` | E: order `pakt`, `desc`, `data` | 5 000 | `cfg05_order_pakt_desc_data` | [x] |
| 6  | `ima_parse` | G=1: one unknown chunk before `desc` | 5 000 | `cfg06_one_unknown_chunk_first` | [x] |
| 7  | `ima_parse` | G=1..8 unknown chunks inserted at random positions around `desc`/`pakt` (D fall-through) | 5 000 | `cfg07_many_unknown_chunks_interleaved` | [x] |
| 8  | `ima_parse` | F: 2..4 `desc` chunks with different values — the **last** before `data` wins | 5 000 | `cfg08_multiple_desc_last_wins` | [x] |
| 8b | `ima_parse` | F+J: 2..4 `desc` chunks where only the **last** has a valid `format_id` (earlier ones invalid) ⇒ still `0` | 3 000 | `cfg08b_multiple_desc_only_last_format_id_matters` | [x] |
| 9  | `ima_parse` | F: 2..4 `pakt` chunks with different `frame_count` — the **last** wins | 5 000 | `cfg09_multiple_pakt_last_wins` | [x] |
| 10 | `ima_parse` | E+F: two `data` chunks (plus a second `desc`/`pakt` after the first `data`) — the **first** `data` wins and the later chunks must be ignored | 2 000 | `cfg10_two_data_chunks_first_wins` | [x] |
| 11 | `ima_parse` | H+I: runs of 1..6 skipped chunks with declared sizes `0..=96`, i.e. strides `16..=112` (size 0 packs chunks back-to-back at the bare 16-byte stride) | 5 000 | `cfg11_positive_skip_sizes` | [x] |
| 12 | `ima_parse` | H: a chunk with size `+80` jumps *forward* over `pakt`/`data`, then a chunk with size `-96` jumps **backwards** onto `pakt`; the scan then reaches `data` | 3 000 | `cfg12_negative_chunk_size_walks_backwards` | [x] |
| 12b| `ima_parse` | H: a `-80` chunk jumps **backwards** directly onto the `data` chunk, which physically precedes the jump chunk | 3 000 | `cfg12b_negative_chunk_size_jumps_back_onto_data` | [x] |
| 13 | `ima_parse` | H+Q: `data` chunk size = `0, ±1, ±2, ±16, i64::MIN(+1), i64::MAX(−1), ±2^32, 0x00FF…, ±0x7F00…` then random; asserts `info->size == ds as u64` | 5 000 | `cfg13_data_chunk_size_extremes` | [x] |
| 14 | `ima_parse` | C+I+O: two files identical except in every byte the C never reads; both libraries must give identical results for both fillings | 5 000 ×2 | `cfg14_unread_bytes_are_ignored` | [x] |
| 15 | `ima_parse` | M: `frame_count` = `0, 1, 2, u64::MAX(−1), i64::MAX, i64::MIN, 0x00FF…, 0xFF00…, 0x0102…, 0x8080…, 2^63(+1), 0xDEADBEEFCAFEBABE` then random | 5 000 | `cfg15_frame_count_values` | [x] |
| 16 | `ima_parse` | N: `data` base pointer at every alignment offset 0..7 | 8 × 2 000 | `cfg16_misaligned_buffer` | [x] |
| 17 | `ima_parse` | J✗: `format_id` = curated near-misses (`ima3`, `ima5`, `IMA4`, `4ami`, …) then random ⇒ `-3` with `*info` provably untouched | 20 000 | `cfg17_bad_format_id_randomized` | [x] |
| 18 | `ima_parse` | L: `channels_per_frame` = `0,1,2,3,4,6,8, 0xFFFFFFFF(−1), 0x7FFFFFFF, 0x80000000, 0x000000FF, 0xFF000000, 0x01020304, 0xDEADBEEF` then random; asserts `info->channel_count == ch` | 5 000 | `cfg18_channel_count_values` | [x] |
| 19 | `ima_parse` | K: `sample_rate` = arbitrary random `u64` bit patterns reinterpreted as `double` (NaN / inf / subnormal / astronomically large) | 30 000 | `cfg19_sample_rate_random_bit_patterns` | [x] |
| 19b| `ima_parse` | K: `sample_rate` biased to the *interesting* magnitudes uniform bit patterns never reach — small ±, fractional, straddling `±2^63` at double granularity, `(0,1)` and `(−1,0)` (truncate to `0`/`-0`) | 30 000 | `cfg19b_sample_rate_biased_magnitudes` | [x] |
| 20 | `ima_parse` | K: 37 curated hard doubles + 15 curated hard bit patterns (`±0.0`, `±1.0`, `±0.5`, `±1.5`, `0.999…`, `44100`, `8000`, `22050`, `±48000.5`, `1e18`, `9.2e18`, `9.3e18`, `1.9e19`, `2^64`, **exactly `2^63`**, largest double `< 2^63`, just above `2^63`, `±2^63`, just below `−2^63`, `±1e300`, `f64::MAX/MIN`, `±MIN_POSITIVE`, `±5e-324`, `±inf`, qNaN, sNaN, NaN payloads), each × 8 noise fillings × alignments 0..7 | 52 × 8 | `cfg20_sample_rate_hard_doubles` | [x] |
| 21 | `ima_parse` | K × L × M × Q cross-product (4 × 3 × 3 × 4 value classes) inside a randomly shaped chunk stream at a random alignment | 20 000 | `cfg21_cross_product` | [x] |
| 22 | `ima_parse` | P: `info->blocks` must equal `data_ptr + data_chunk_offset + 20`, with the `data` chunk pushed to a different offset every iteration by 0..6 random skipped chunks, at random alignment | 5 000 | `cfg22_blocks_pointer_identity` | [x] |
| 23 | `ima_parse` | end-to-end fuzz of the composed pipeline: random magic (5 % invalid), random version (5 % invalid), random `format_id` (12 % invalid), random ordering, 0..5 unknown chunks, random sizes and alignment | 30 000 | `cfg23_whole_file_fuzz` | [x] |
| 24 | `ima_parse` | E × J interaction: `desc` present with an **invalid** `format_id` **and no `pakt` at all** ⇒ `-3` is returned before the NULL `pakt` would be dereferenced | 3 000 | `cfg24_bad_format_id_without_pakt` | [x] |
| 25 | `ima_parse` | I: **overlapping** chunk headers — the `desc` chunk declares size 8 but has a 32-byte payload, so the next chunk header is parsed *out of the desc payload* (type = `format_id` = `"ima4"`, size = the `bytes_per_packet`/`frames_per_packet` pair) and lands exactly after it | 3 000 | `cfg25_overlapping_chunk_headers` | [x] |

**Total: ~430 000 differential `ima_parse` invocation pairs.**

## Result

```
$ cargo test --release --test phase_b_valid
test result: ok. 29 passed; 0 failed
```

All rows pass under both the `dev` and `release` profiles and under all three
Cargo feature configurations (see `verify.sh`).
