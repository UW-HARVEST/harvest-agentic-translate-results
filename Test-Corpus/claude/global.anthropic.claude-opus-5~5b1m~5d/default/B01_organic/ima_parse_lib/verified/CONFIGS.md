# CONFIGS.md — Phase A configuration-surface table (valid inputs)

## Public entry points

`c_src/include/lib.h` declares exactly one function:

```c
int ima_parse(struct ima_info *info, const void *data);
```

There is no options struct, no init/free pair, no mode flag, no `#ifdef` in the
C source, and the crate declares **no cargo features** (`Cargo.toml` has no
`[features]` section). Therefore the configuration surface is entirely the
*shape of the input byte buffer* `data`, plus the `struct ima_info` output
layout. The axes below are derived from every `if` / `else if` / loop-update in
`ima_parse`, and from every field the function loads.

## Axes the C actually branches on

| axis | values the C distinguishes |
|---|---|
| A1 `header->type` (BE FourCC @0) | `caff` (pass) / anything else (`-1`) |
| A2 `header->version` (BE u16 @4) | `1` (pass) / anything else (`-2`) |
| A3 `header->flags` (BE u16 @6) | **never read** — must be ignored (fuzz it) |
| A4 chunk type (BE FourCC @chunk+0) | `desc` / `pakt` / `data` (breaks) / anything else (skipped) |
| A5 `chunk->size` (BE s64 @chunk+8) | 0 / positive / negative (cursor moves backwards) / huge |
| A6 chunk **stride** | `sizeof(struct caf_chunk)` == **16** (not 12) because of the 4-byte padding after `type`; cursor advance is `chunk + 16 + size` |
| A7 chunk order / multiplicity | `desc`→`pakt`→`data`, `pakt`→`desc`→`data`, duplicate `desc` (last wins), duplicate `pakt` (last wins), unknown chunks interleaved, 0 unknown / 1 / many |
| A8 `desc->format_id` (BE @desc+8) | `ima4` (pass) / else `-3` |
| A9 `desc->sample_rate` (raw 8 bytes @desc+0) | fed through native `double` load → `(u64)` value conversion → `bswap64` → bit-reinterpret. Distinguished sub-ranges: `[0,1)`→0, `[1,2^63)`→truncation, `>= 2^63`→`subsd`+`xor` path, negative in `(-1,0]`→0, `<= -1`→`cvttsd2si` negative, `< -2^63`/`+Inf`/`-Inf`/`NaN`→`0x8000000000000000` |
| A10 `desc->channels_per_frame` (BE u32 @desc+24) | 0 / 1 / 2 / large / `0xFFFFFFFF` |
| A11 `pakt->frame_count` (BE s64 @pakt+8) | 0 / positive / negative / `i64::MIN` / `u64::MAX` |
| A12 `blocks` output pointer | always `data_chunk + 16 + sizeof(caf_data)==4` → `+20`; must be byte-identical pointer |
| A13 `info->size` output | the **`data` chunk's** `chunk_size` (the loop variable at break), reinterpreted as `u64` |
| A14 buffer/`data` alignment | `data` 8-aligned / 4-aligned / 2-aligned / 1-aligned (odd address) — the C reads through casts, so unaligned loads must behave the same |
| A15 fields never read | `chunk` types other than the three; `desc->format_flags`, `bytes_per_packet`, `frames_per_packet`, `bits_per_channel`; `pakt->packet_count`, `priming_frames`, `remainder_frames`; `caf_data->edit_count`; `ima_block` contents — all must be ignored (fuzz them) |

## Row table (pruned cross-product — combinations the C treats differently)

Every row is driven with **many randomized inputs** (fixed seed, `SplitMix64`),
not a single hand-picked value, and asserts the full 40-byte `struct ima_info`
plus the `int` return are byte-identical between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ima_parse` | minimal valid: `desc`(ima4) → `pakt` → `data`; all sizes = real payload size; randomized `flags`, `format_flags`, `bytes/frames_per_packet`, `bits_per_channel`, `packet_count`, `priming`, `remainder`, `edit_count` (A3, A15) | [x] |
| 2 | `ima_parse` | reordered: `pakt` → `desc` → `data` (A7) | [x] |
| 3 | `ima_parse` | unknown chunks interleaved: 0, 1, 2, 8 unknown chunks with random FourCCs (excluding the 3 known) before/between/after `desc`/`pakt` (A4, A7) | [x] |
| 4 | `ima_parse` | duplicate `desc` chunks (second, later one must win — `desc` is overwritten) (A7) | [x] |
| 5 | `ima_parse` | duplicate `pakt` chunks (later one wins) (A7) | [x] |
| 6 | `ima_parse` | unknown chunk with `size == 0` → cursor advances by exactly 16 (A5, A6) | [x] |
| 7 | `ima_parse` | unknown chunk with large positive `size` (skips a big padded region) (A5) | [x] |
| 8 | `ima_parse` | **negative** `chunk_size` on an unknown chunk → cursor walks backwards and re-visits an earlier chunk; laid out so the scan still terminates at `data` (A5, ERRORS row 7) | [x] |
| 9 | `ima_parse` | `data` chunk `size` = 0 / small / large / `0x7FFF_FFFF_FFFF_FFFF` / negative / `0xFFFF_FFFF_FFFF_FFFF` → `info->size` (A5, A13) | [x] |
| 10 | `ima_parse` | `sample_rate` raw bytes = the big-endian encoding of a *realistic* rate (8000, 11025, 22050, 32000, 44100, 48000, 96000, 192000) — the usual case, where the mis-read subnormal truncates to 0 (A9) | [x] |
| 11 | `ima_parse` | `sample_rate` raw bytes chosen so the **native** load lands in `[1, 2^63)` → non-zero truncation path (A9) | [x] |
| 12 | `ima_parse` | `sample_rate` raw bytes chosen so the native load is `>= 2^63` → `subsd`/`xor 1<<63` path (A9) | [x] |
| 13 | `ima_parse` | `sample_rate` raw bytes = negative double: `(-1,0)`, `[-2^63,-1]`, `< -2^63` (A9) | [x] |
| 14 | `ima_parse` | `sample_rate` raw bytes = `+Inf`, `-Inf`, quiet NaN, signalling NaN, `-0.0`, subnormals, `f64::MAX`, `f64::MIN_POSITIVE` (A9) | [x] |
| 15 | `ima_parse` | `sample_rate` = fully random 64 bits (10 000 iterations) — the exhaustive sweep of A9 | [x] |
| 16 | `ima_parse` | `channels_per_frame` = 0 / 1 / 2 / 6 / 0x7FFFFFFF / 0xFFFFFFFF / random (A10) | [x] |
| 17 | `ima_parse` | `frame_count` = 0 / 1 / `i64::MAX` / `-1` / `i64::MIN` / random 64 bits (A11) | [x] |
| 18 | `ima_parse` | `blocks` pointer identity: same buffer address passed to both libraries, assert both return the *same* `blocks` pointer value = `&data_chunk + 20` (A12) | [x] |
| 19 | `ima_parse` | misaligned `data`: buffer offset by 1..7 bytes from an 8-aligned address, full valid parse (A14) | [x] |
| 20 | `ima_parse` | `ima_block` payload region filled with random bytes; `data` chunk followed by 0 / 1 / many blocks — contents must be ignored, `blocks` pointer only (A12, A15) | [x] |
| 21 | `ima_parse` | full random valid document: random chunk order, random unknown-chunk count/sizes, random every unread field, random `sample_rate` bits — 5 000 iterations, structural fuzz (all axes) | [x] |
| 22 | `ima_parse` | `desc` chunk present but `data` chunk found **first** in file order while `desc` appears later — reached via a negative-size back-jump so `desc` *is* set before `data` is hit (A5, A7) | [x] |
| 23 | `ima_parse` | `info` struct pre-filled with a random poison pattern before the call → on the `-1`/`-2`/`-3` paths the struct must be left byte-identical to the poison in both libraries (A1, A2, A8) | [x] |
| 24 | `ima_parse` | called repeatedly on the same `info` (statelessness): 100 alternating valid/invalid calls, comparing `info` after each (no hidden global state in either library) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`** section, so the only
build configurations are: default features, `--no-default-features`, and
`--all-features` — all three are identical. All are run by
`run_all_features.sh` for completeness.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`. There is
no `add_executable`, and `translation/Cargo.toml` declares no `[[bin]]`.
**No driver binary exists**, so the "compare stdout" gate is not applicable.

---

## Phase B results

All 24 rows pass. `cargo test --release` → `phase_b_configs`: **24 passed, 0 failed**.
Total randomized cases driven through both `.so`s across the rows: ~25 000
documents, each compared on the `int` return value **and** all 40 bytes of
`struct ima_info` (twice, with two different poison patterns for the output
struct). No divergence was found on any valid-path row.

Notable properties confirmed byte-for-byte against the C:

* the chunk stride really is `sizeof(struct caf_chunk) == 16` (row 6), not the
  12 bytes the CAF file format specifies — the C's struct padding is reproduced;
* `blocks` is exactly `&data_chunk + 16 + 4` (rows 18, 22);
* `info->size` is the `data` chunk's size field, sign-extended through
  `ima_s64_t` and re-widened to `ima_u64_t` (row 9);
* the `double -> unsigned long long` *value* conversion of the raw sample-rate
  bits (not a bit-reinterpret) matches GCC's x86-64 lowering
  (`comisd 2^63 / jae / subsd / cvttsd2si / xor 1<<63`) for every input class,
  including NaN, ±Inf, ≥2^63 and < −2^63 (rows 11–15, 10 000 random bit
  patterns);
* negative `chunk_size` walking the cursor *backwards* is followed, not rejected
  (rows 8, 22);
* unaligned `data` pointers behave identically (row 19).

## Phase D results

`./run_all_features.sh` builds **4 C variants** (CMake default / Release −O3 /
RelWithDebInfo −O2 / Debug −g) × **3 feature combos** (default,
`--no-default-features`, `--all-features`) × **2 Rust profiles**
(release, dev) = **24 configurations**, and for each one runs the entire Phase B
+ Phase C suite plus the `nm -D` symbol diff and an `ldd -r` unresolved-symbol
check.

Result: `ALL CONFIGURATIONS PASSED`, reproduced on 3 consecutive full runs.
Symbol diff is empty in every configuration (1 exported symbol, `ima_parse`);
`ldd -r` reports 0 unresolved symbols.
