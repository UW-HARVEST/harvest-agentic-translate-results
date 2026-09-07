# CONFIGS.md — configuration-surface table (Phase B gate)

## Axes, derived from the C source

### Entry points (the FULL public set)

`c_src/include/lib.h` declares exactly one function, and it is also the lowest
level entry point — there is no convenience wrapper / one-shot layer to hide
behind:

* `int ima_parse(struct ima_info *info, const void *data)`

The six byte-swap helpers (`ima_bswap16/32/64`, `ima_btoh16/32/64`) are `static`
in C and unreachable from outside. They are exercised *transitively*, on every
field, by the rows below (each row's random field values drive `ima_btoh16`,
`ima_btoh32` and `ima_btoh64` over random inputs).

### Runtime options / modes / flags

There are **no** runtime option arguments, no flags parameters, no global
configuration state, no `#ifdef`/`#if` in the source and no compile-time
defines in `CMakeLists.txt`, and the Rust crate declares no cargo `[features]`.
The only "configuration" the library reacts to is therefore encoded in the
**input buffer shape**, enumerated below.

### Input shapes the C actually branches on (`grep 'if\|else\|for' src/lib.c`)

| axis | distinct values the C distinguishes |
|------|--------------------------------------|
| `header.type` (BE u32) | `'caff'` / anything else (→ `-1`) |
| `header.version` (BE u16) | `1` / anything else (→ `-2`) |
| `header.flags` | **never read** — must be ignored (fuzzed to prove it) |
| chunk `type` (BE u32) | `'desc'`, `'pakt'`, `'data'` (terminates loop), *anything else* (skipped by `size`) |
| chunk `size` (BE i64) | `0`, positive, **negative** (pointer walks backwards), huge; only the *last* (`data`) one reaches `info->size` |
| chunk ordering | `desc`→`pakt`→`data`, `pakt`→`desc`→`data`, unknown chunks interleaved anywhere |
| chunk multiplicity | 0, 1, many `desc`; 0, 1, many `pakt` (last occurrence wins); ≥1 unknown |
| `desc.format_id` (BE u32) | `'ima4'` / anything else (→ `-3`) |
| `desc.sample_rate` | raw 8 bytes → native `double` → **arithmetic** `double`→`u64` conversion (`lib.c:127`): distinct code paths for `[0,2^63)`, `(−2^63,0)`, `≥2^63`, `<−2^63`, `NaN`, `±Inf`, `±0`, subnormal |
| `desc.channels_per_frame` | any `u32` (no check) — `0`, `1`, `2`, `0xFFFFFFFF`, random |
| `desc.format_flags`/`bytes_per_packet`/`frames_per_packet`/`bits_per_channel` | **never read** — must be ignored (fuzzed to prove it) |
| `pakt.frame_count` (BE i64) | any value — `0`, `−1`, `i64::MIN`, `i64::MAX`, random |
| `pakt.packet_count`/`priming_frames`/`remainder_frames` | **never read** — must be ignored (fuzzed to prove it) |
| `data` pointer alignment | 8-aligned, and offsets 1..7 (all loads become unaligned) |
| `info` pointer | valid / `NULL` (see `ERRORS.md` E13/E14) |
| output `info->blocks` | derived pointer = `&data_chunk + 16 + sizeof(caf_data=4)`; must be the *same address* from both libraries |

### C ABI layouts the pointer arithmetic depends on (must match exactly)

| struct | size | align | field offsets used |
|--------|------|-------|--------------------|
| `caf_header` | 8 | 4 | `type`@0, `version`@4 |
| `caf_chunk` | **16** | 8 | `type`@0, (4 pad), `size`@8 |
| `caf_audio_description` | 32 | 8 | `sample_rate`@0, `format_id`@8, `channels_per_frame`@24 |
| `caf_packet_table` | 24 | 8 | `frame_count`@8 |
| `caf_data` | 4 | 4 | — (only its size matters) |
| `ima_block` | 34 | 2 | — |
| `ima_info` | 40 | 8 | `blocks`@0, `size`@8, `sample_rate`@16, `frame_count`@24, `channel_count`@32 |

## Rows — one per meaningful combination the C treats differently

Each row is driven with **many randomized inputs** (fixed seed, SplitMix64 PRNG,
`ITERS` per row) and compared field-by-field / byte-for-byte between the C `.so`
and the Rust `.so`, both loaded via `libloading`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `ima_parse` | canonical minimal valid file: `desc`(size 32) → `pakt`(size 24) → `data`; all read fields randomized, unread fields randomized too | [x] |
| C2 | `ima_parse` | order swapped: `pakt` → `desc` → `data` | [x] |
| C3 | `ima_parse` | one unknown chunk *before* `desc`, declared `size` == its real payload size (random 0..256) | [x] |
| C4 | `ima_parse` | unknown chunk with `size == 0` (loop advances exactly 16 bytes) | [x] |
| C5 | `ima_parse` | unknown chunk whose declared `size` is *larger* than needed → skips over trailing padding | [x] |
| C6 | `ima_parse` | `desc` chunk whose declared `size` > 32 (padding after the payload); `pakt` likewise > 24 | [x] |
| C7 | `ima_parse` | unknown chunk with **negative** `size`: the walk moves backwards, then a second copy of the chunk list is reached (pointer arithmetic parity) | [x] |
| C8 | `ima_parse` | duplicate `desc` chunks (2..4 of them, different payloads) → **last** wins | [x] |
| C9 | `ima_parse` | duplicate `pakt` chunks (2..4) → **last** wins | [x] |
| C10 | `ima_parse` | many chunks: 8..64 unknown chunks interleaved with `desc`/`pakt` before `data` (deep loop) | [x] |
| C11 | `ima_parse` | `data` chunk `size` shape: `0` / small positive / `2^32` / `i64::MAX` / `-1` / `i64::MIN` / random `i64` → `info->size` | [x] |
| C12 | `ima_parse` | `sample_rate` = random `u64` **bit pattern** (the realistic case: BE-encoded bytes read as native LE `double`) — hits the whole UB domain of the `double`→`u64` conversion | [x] |
| C13 | `ima_parse` | `sample_rate` = a real `f64` in `[0, 2^63)` (in-range truncation path), incl. typical rates 8000/22050/44100/48000/96000 and random magnitudes | [x] |
| C14 | `ima_parse` | `sample_rate` = a real `f64` in `(−2^63, 0)` (negative `cvttsd2si` path) | [x] |
| C15 | `ima_parse` | `sample_rate` ≥ 2^63 finite (the `subsd`+`btc` path), incl. exactly `2^63`, `2^63+1024`, `1e300`, `f64::MAX` | [x] |
| C16 | `ima_parse` | `sample_rate` = `< −2^63` finite, `±Inf`, quiet NaN, signalling NaN (random payloads), `±0.0`, subnormal, `f64::MIN_POSITIVE`, `2^63−1024`, `−2^63` | [x] |
| C17 | `ima_parse` | `channel_count` axis: `0`, `1`, `2`, `0xFFFF`, `0xFFFFFFFF`, random `u32` | [x] |
| C18 | `ima_parse` | `frame_count` axis: `0`, `1`, `−1`, `i64::MIN`, `i64::MAX`, random `i64` | [x] |
| C19 | `ima_parse` | unaligned `data` base pointer, offsets 1..7, otherwise canonical (unaligned field loads) | [x] |
| C20 | `ima_parse` | `blocks` output-pointer parity: `data` chunk placed at a random offset in the buffer; asserts both libs return the identical derived address | [x] |
| C21 | `ima_parse` | never-read fields (`header.flags`, `desc.format_flags`, `bytes_per_packet`, `frames_per_packet`, `bits_per_channel`, `pakt.packet_count`, `priming_frames`, `remainder_frames`) filled with random noise — output must be unaffected | [x] |
| C22 | `ima_parse` | full structural fuzz: randomized number/order/size/type of chunks, random payloads, random alignment, random `info` — one PRNG drives everything (10 000 cases) | [x] |
| C23 | `ima_parse` | `data` chunk is the **first** chunk but `desc`/`pakt` precede it in a *previous* backwards jump (combination of C7 + C1) | [x] |

## Binary / driver

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)` and
no `add_executable`. `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` and has no `[[bin]]` and no `src/main.rs`.
**The project builds no binary executable, so the stdout-comparison item is
not applicable.**

## Feature combinations

`translation/Cargo.toml` has no `[features]` section ⇒ the only combination is
the default (empty) feature set. Verified by the loop in
`scripts/check_features.sh`, which enumerates features from `Cargo.toml` and
runs `cargo check`/`cargo test` over the power set.

## Divergence found and fixed during Phase D

`crash::crash_parity` (see `ERRORS.md` E14/E15) caught a real divergence in the
**dev-profile** cdylib:

* `(*info).blocks = blocks` (`src/lib.rs:307`) and the `read_unaligned` in
  `load::<T>` hit Rust's `debug_assertions`-gated UB checks when `info` /
  `data` is NULL. The check panics with `"null pointer dereference occurred"`;
  a panic cannot unwind out of `extern "C"`, so the process died with
  **SIGABRT (6)** where the C dies with **SIGSEGV (11)**.
* The C intentionally dereferences both pointers without a NULL check and walks
  the chunk list with attacker-controlled (possibly negative) offsets, so those
  checks must be off for the translation to be faithful.
* Fix: `debug-assertions = false` / `overflow-checks = false` in **both**
  `[profile.dev]` and `[profile.release]` in `Cargo.toml`. After the fix all
  four crash cases die with SIGSEGV in both libraries, in both profiles.

## Robustness: C optimization level

The ground-truth build (`c_src/CMakeLists.txt`) compiles with `-fPIC` and no
`-O` flag. The whole suite was additionally re-run against the C compiled at
`-O0`, `-O1`, `-O2`, `-O3` and `-Os` — **all pass**, so the `double`→`u64`
emulation is not tied to one codegen choice.

`-Ofast` is the one exception and is *expected*: `-ffinite-math-only` lets GCC
assume `desc->sample_rate` is never NaN, so it drops the `comisd`/NaN branch of
the conversion and yields `0` instead of `0x8000_0000_0000_0000`. Only the NaN
inputs differ, and only under fast-math, which the project does not use.
