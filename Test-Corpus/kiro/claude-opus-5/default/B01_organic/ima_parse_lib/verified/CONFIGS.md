# CONFIGS.md — configuration-surface table (Phase A, gate for Phase B)

## How this was derived

The public header `c_src/include/lib.h` exposes **one** entry point,
`ima_parse`, and **no** options struct, flags word, mode enum, setter, context
object or init/teardown pair. `grep -n '#if\|#ifdef\|#ifndef\|#define' c_src/**`
finds no compile-time switches; `translation/Cargo.toml` has **no `[features]`
section**, so there is exactly **one** build configuration.

Therefore all runtime configuration is carried **in the input byte stream**.
The axes below are exactly the things `ima_parse` branches on or reads:

| axis | values the C actually distinguishes | source |
|---|---|---|
| A. header magic (bytes 0..4, BE) | `caff` / anything else | `if (ima_btoh32(header->type) != 'caff') return -1` |
| B. header version (bytes 4..6, BE) | `1` / anything else | `if (ima_btoh16(header->version) != 1) return -2` |
| C. header flags (bytes 6..8) | never read → must not matter | field unused |
| D. chunk type (`chunk->type`, BE u32 @ chunk+0) | `desc` / `pakt` / `data` / *anything else (skip)* | the `if / else if / else if` chain |
| E. chunk size (`chunk->size`, BE i64 @ chunk+**8**, note the 4 bytes of C padding) | `0` / positive / negative / `i64::MIN` / `i64::MAX` | `chunk = (u8*)&chunk[1] + chunk_size` |
| F. chunk ordering | `desc` before `pakt` / `pakt` before `desc` | `desc`/`pakt` are latched, `data` breaks |
| G. chunk multiplicity | 0 / 1 / many unknown chunks interleaved; duplicate `desc`; duplicate `pakt` (**last wins**) | plain assignment, not `if (!desc)` |
| H. `desc->format_id` (BE u32 @ desc+8) | `ima4` / anything else | `if (... != 'ima4') return -3` |
| I. `desc->sample_rate` (native-endian f64 @ desc+0) | `0.0`, `-0.0`, normal, subnormal, negative, fractional, `≥2^63`, `<-2^63`, ±Inf, NaN (quiet/signalling), exact `2^63` | arithmetic `double→u64`, then bswap64, then bit-reinterpret as `double` |
| J. `desc->channels_per_frame` (BE u32 @ desc+24) | `0` / `1` / `2` / `0xFFFFFFFF` / random | copied through |
| K. `pakt->frame_count` (BE i64 @ pakt+8) | `0` / `1` / negative / `i64::MIN` / `i64::MAX` / random | copied through |
| L. `data` chunk `size` | `0` / positive / negative / extremes | becomes `info->size` |
| M. buffer alignment of `data` | 8-byte aligned / offsets 1..15 | plain (unaligned-tolerant) x86-64 loads |
| N. `info` pointer | valid / NULL-on-error-path / **aliasing the input buffer** | only written on success, and the writes are interleaved with the reads of `desc`/`pakt`, so aliasing makes the write order observable |
| O. absolute buffer address | must not affect anything except the returned `blocks` pointer, which is `data + <offset of data payload> + 4` | pointer arithmetic |

Unused / never-read fields that must be provably irrelevant (each is filled
with random bytes in every valid-path row): `caf_header.flags`,
`caf_audio_description.format_flags`, `.bytes_per_packet`, `.frames_per_packet`,
`.bits_per_channel`, `caf_packet_table.packet_count`, `.priming_frames`,
`.remainder_frames`, `caf_data.edit_count`, and everything after the `data`
chunk payload start.

## Rows

Every row is exercised through **both** `.so`s via `libloading` with **256
randomized inputs** (fixed seed, `SplitMix64`) for all axes not pinned by the
row, comparing the returned `int` **and** all five `ima_info` fields
byte-for-byte (`sample_rate` compared by raw `to_bits()`, `blocks` compared as
an offset from the buffer base **and** as an absolute pointer).

Tests live in `translation/tests/configs.rs`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `ima_parse` | minimal valid file: `desc` then `pakt` then `data`; all chunk sizes exact; random unused fields | [x] |
| C02 | `ima_parse` | `pakt` before `desc` (F reversed) | [x] |
| C03 | `ima_parse` | `data` chunk is the very first chunk after `desc`+`pakt`, zero unknown chunks (G=0) | [x] |
| C04 | `ima_parse` | exactly one unknown chunk (random FourCC ∉ {desc,pakt,data}) between `desc` and `pakt` (D=skip, G=1) | [x] |
| C05 | `ima_parse` | 2..8 unknown chunks interleaved at random positions, random sizes 0..64 (G=many) | [x] |
| C06 | `ima_parse` | unknown chunk with `size == 0` (E=0) — walk advances by exactly 16 | [x] |
| C07 | `ima_parse` | unknown chunk with a large positive `size` (up to 4096, buffer sized to match) | [x] |
| C08 | `ima_parse` | unknown chunk with **negative** `size` that lands the walk back onto a valid chunk (E<0, G5) | [x] |
| C09 | `ima_parse` | duplicate `desc` chunks — last one wins (G) | [x] |
| C10 | `ima_parse` | duplicate `pakt` chunks — last one wins (G) | [x] |
| C11 | `ima_parse` | duplicate `data`-typed FourCC is impossible to exercise twice (first `data` breaks) — asserted by placing a second `data` chunk after the first and checking it is ignored | [x] |
| C12 | `ima_parse` | `data` chunk `size` = `0` (L=0) | [x] |
| C13 | `ima_parse` | `data` chunk `size` = random positive `u32` (L) | [x] |
| C14 | `ima_parse` | `data` chunk `size` = `-1`, `i64::MIN`, `i64::MAX` (L extremes; `info->size` is the raw bit pattern) | [x] |
| C15 | `ima_parse` | `sample_rate` = canonical audio rates `8000, 11025, 22050, 32000, 44100, 48000, 88200, 96000, 192000` (I normal) | [x] |
| C16 | `ima_parse` | `sample_rate` = `0.0` and `-0.0` (I) | [x] |
| C17 | `ima_parse` | `sample_rate` = fractional values `0.5, 1.5, -0.5, 44100.7, 1e-300` (truncation toward zero) | [x] |
| C18 | `ima_parse` | `sample_rate` = negative integral values `-1.0, -44100.0, -9e18` (I negative → conversion wraps) | [x] |
| C19 | `ima_parse` | `sample_rate` = exactly `2^63`, `2^63-1024`, `2^63+2^11`, `2^64`, `1e300` (I ≥2^63 → the `subsd`/`xor` codegen path) | [x] |
| C20 | `ima_parse` | `sample_rate` = `-2^63`, `-2^63-2^11`, `-1e300` (below signed range) | [x] |
| C21 | `ima_parse` | `sample_rate` = `+Inf`, `-Inf` (I) | [x] |
| C22 | `ima_parse` | `sample_rate` = quiet NaN, signalling NaN, NaN with random payload, negative NaN (I) | [x] |
| C23 | `ima_parse` | `sample_rate` = subnormals (`f64::MIN_POSITIVE/2`, smallest subnormal, negative subnormal) | [x] |
| C24 | `ima_parse` | `sample_rate` = fully random 64-bit patterns reinterpreted as `f64` (covers the whole of axis I) | [x] |
| C25 | `ima_parse` | `channels_per_frame` = `0`, `1`, `2`, `6`, `0xFFFFFFFF`, random (J) | [x] |
| C26 | `ima_parse` | `frame_count` = `0`, `1`, `-1`, `i64::MIN`, `i64::MAX`, random (K) | [x] |
| C27 | `ima_parse` | header `flags` = `0x0000`, `0xFFFF`, random (C — must not change output) | [x] |
| C28 | `ima_parse` | all "unused" `desc`/`pakt`/`caf_data` fields set to `0xFF…` vs random vs zero (must not change output) | [x] |
| C29 | `ima_parse` | `data` buffer start unaligned by 1..15 bytes (M) | [x] |
| C30 | `ima_parse` | buffer allocated at many different absolute addresses; `blocks` must equal `base + payload_offset + 4` in both (O) | [x] |
| C31 | `ima_parse` | chunk payloads larger than the struct the C reads (`desc` chunk `size` > 32, `pakt` chunk `size` > 24) — extra bytes ignored, walk still uses `size` | [x] |
| C32 | `ima_parse` | chunk payloads **smaller** than the struct the C reads (`desc` `size` = 0 while a full 32-byte `desc` body follows in the buffer) — the C reads past the declared size; must match | [x] |
| C33 | `ima_parse` | full cross-product smoke: random valid file with every axis randomized simultaneously, 4096 iterations (fixed seed) | [x] |
| C34 | `ima_parse` | `desc`/`pakt`/`data` chunk types with one byte perturbed → treated as unknown/skip (D boundary, G4) | [x] |
| C35 | `ima_parse` | unknown chunk FourCCs exhaustively probed at the 3 known values ±1 in every byte, plus `0x00000000` and `0xFFFFFFFF` | [x] |
| C36 | internal helpers `ima_bswap16/32/64`, `ima_btoh16/32/64` | `static` in C ⇒ no ABI surface. Covered **indirectly and exhaustively**: every u32/u64/u16 field read in rows C01–C35 goes through them, and rows C24/C26/C35 feed fully random 64/32-bit values. No direct row is possible without modifying `c_src/`. | [x] |
| C37 | `ima_parse` | `info` **aliases the input buffer** at every 4-byte offset plus several unaligned ones (N). The C writes `info->blocks` and `info->size` *before* reading `pakt->frame_count`, `desc->channels_per_frame` and `desc->sample_rate`, so the write/read interleaving is observable. Both implementations are run against the **same allocation at the same address** (restored in between) and the whole buffer is compared byte-for-byte afterwards. | [x] |
| C37b | `ima_parse` | `info` aliases the input buffer on the `-1` / `-2` / `-3` error paths — nothing may be written, so the buffer must come back untouched | [x] |
| C24b | `ima_parse` | dedicated high-volume sweep of the `double → u64 → bswap64 → double` pipeline (the only non-obvious computation in the library): 150k uniform random 64-bit patterns, 40 passes over every one of the 2048 exponents with random mantissas in both signs, and dense ±4096-ULP sweeps around `2^63`, `-2^63`, `2^64`, `1.0` and `±0.0` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`**, so the complete set of
cargo feature combinations is `{ default }` — which is also `{ --no-default-features }`.
`run_tests.sh` derives the list mechanically from `Cargo.toml` (so it stays
correct if features are ever added) and runs the whole suite for each
combination in **both** the `dev` and `release` profiles.

## Suite self-validation (mutation testing)

The suite was validated by injecting 11 deliberate mistranslations into
`src/lib.rs`, rebuilding the `.so` and re-running. **All 11 were caught**:

| mutation | caught by |
|---|---|
| M1 `sample_rate`: bit-reinterpret instead of arithmetic `double→u64` | C15–C24b |
| M2 `blocks`: drop the `caf_data` offset (chunk+16 instead of chunk+20) | C01/C08/C11/C30 |
| M3 chunk header 12 bytes instead of 16 (the real CAF layout) | C01 and most others |
| M4 version compared native-endian instead of big-endian | E2d/E2e |
| M5 error code `-1` returned where `-2` belongs | E1*/C37b |
| M6 `desc`/`pakt`: first match wins instead of last | C09/C10/E3g |
| M7 `cvttsd2si` NaN result `0` instead of `0x8000…` | C22/C24b |
| M8 `channels_per_frame` read at offset 20 instead of 24 | C25 |
| M9 `format_id` read at offset 12 instead of 8 | E3*/C01 |
| M10 `frame_count` read at `packet_count`'s offset | C26/C10 |
| M11 `sample_rate` written before the other fields (order change only) | **C37** |

M11 is only observable through the aliasing row C37, which is why that row
exists.
