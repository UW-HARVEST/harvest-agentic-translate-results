# CONFIGS.md — Phase A, step 3: CONFIGURATION-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h` +
`c_src/CMakeLists.txt`.

## Build-time configuration axes

| source | axes found |
|--------|-----------|
| `Cargo.toml` | **no `[features]` section** → exactly one feature combination: `--no-default-features` (empty set), which is identical to the default build |
| `c_src/CMakeLists.txt` | no `option()`, no `if()`, no `add_definitions`, no `target_compile_definitions` → exactly one C configuration (single TU `src/lib.c`, `SHARED` library) |
| `src/lib.rs` | no `#[cfg(feature = ...)]`, no `#[cfg(...)]` at all |

**Total valid feature combinations: 1** (the empty feature set == default).

## Runtime configuration axes

There is no state, no init/teardown, no option setter, no global, no mode flag.
The single public entry point is a pure function of the bytes it reads:

| public entry point | header | level |
|--------------------|--------|-------|
| `int hdr_compare(const uint8_t *h1, const uint8_t *h2)` | `lib.h` | the lowest-level (and only) exported entry point — there is no convenience wrapper to hide behind |
| `static int hdr_valid(const uint8_t *h)` | not in header | internal; reachable **only** as `hdr_valid(h2)` from `hdr_compare`, so every `hdr_valid` axis is driven through the `h2` argument |

Therefore the "configuration" of a call is entirely the **shape of the two
3-byte header inputs**. The axes the C actually branches on (one axis per
`if`/`&&`/`||`/mask in the source):

| axis | field | C expression | distinct values the C treats differently |
|------|-------|--------------|------------------------------------------|
| A1 | `h2[0]` sync byte 1 | `h[0] == 0xff` | `0xff` / any of the other 255 |
| A2 | `h2[1]` sync bits + version | `(h[1] & 0xF0) == 0xf0` \|\| `(h[1] & 0xFE) == 0xe2` | MPEG1/2 form (`0xF0..0xFF`), MPEG2.5 form (`0xE2`,`0xE3`), neither (238 values) |
| A3 | `h2[1]` layer | `((h[1] >> 1) & 3) != 0` | layer code `0` (reserved) / `1` / `2` / `3` |
| A4 | `h2[1]` bit 0 (CRC/protection) | *not read by any check*; masked out by `0xFE` | `0` / `1` (must be irrelevant) |
| A5 | `h2[2]` bitrate index | `(h[2] >> 4) != 15` and `(h[2] & 0xF0) == 0` | `0` (free format) / `1..14` (normal) / `15` (reserved) |
| A6 | `h2[2]` sample-rate index | `((h[2] >> 2) & 3) != 3` | `0` / `1` / `2` / `3` (reserved) |
| A7 | `h2[2]` bits 1..0 (padding, private) | *not read by any check*; masked out | `0..3` (must be irrelevant) |
| B1 | `h1[1]` vs `h2[1]` | `((h1[1] ^ h2[1]) & 0xFE) == 0` | equal-under-mask / differing-under-mask |
| B2 | `h1[1]` bit 0 | masked out by `0xFE` | equal / differing (must be irrelevant) |
| B3 | `h1[2]` vs `h2[2]` sample-rate bits | `((h1[2] ^ h2[2]) & 0x0C) == 0` | equal / differing |
| B4 | `h1[2]` vs `h2[2]` free-format flags | `!(((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0))` | both free / both non-free / exactly one free |
| B5 | `h1[2]` bits 1..0 and bitrate value | not compared beyond B4 | differing bitrate values (both non-zero) must still match |
| B6 | `h1[0]` | **never read** | any value (incl. `0x00`) must not change the result |
| B7 | `h1`/`h2` byte 3+ | **never read** | any trailing bytes must not change the result |
| C1 | buffer length / read extent | short-circuit order of `&&` | 1, 2, 3 readable bytes; `h1` untouched when `hdr_valid(h2)` is false |
| C2 | pointer aliasing | none in C | `h1 == h2` (same pointer), overlapping, disjoint |
| C3 | pointer alignment | `uint8_t` reads, alignment-agnostic | offset 0/1/2/3 within an allocation |

## The table — one row per meaningful COMBINATION

Each row is checked with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, SplitMix64) over the free bytes of the row, plus the
exhaustive enumerations noted. `h2v` = a valid `h2`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `hdr_compare` | **identical headers**, `h2` valid: MPEG1 (`h2[1] & 0x18 == 0x18`), layer 3/2/1 (A3∈{1,2,3}), bitrate 1..14, sample-rate 0..2, random A4/A7 → expect `1` | [x] |
| 2 | `hdr_compare` | identical headers, `h2` valid: MPEG2 (`h2[1] & 0x18 == 0x10`), layer 1..3, bitrate 1..14, sample-rate 0..2 → expect `1` | [x] |
| 3 | `hdr_compare` | identical headers, `h2` valid: **MPEG2.5** form `h2[1] ∈ {0xE2,0xE3}` (A2 branch B; layer is forced to 1), bitrate 1..14, sample-rate 0..2 → expect `1` | [x] |
| 4 | `hdr_compare` | identical headers, `h2` valid with **free-format** bitrate index `0` (A5=0), all sample-rates 0..2, all sync/layer forms → expect `1` (both sides free) | [x] |
| 5 | `hdr_compare` | `h2` valid, `h1` differs **only in A4/B2** (CRC bit `h[1] & 1`) → expect `1` (bit masked out) | [x] |
| 6 | `hdr_compare` | `h2` valid, `h1` differs **only in A7/B5 low bits** (`h[2] & 0x03`: padding + private) → expect `1` | [x] |
| 7 | `hdr_compare` | `h2` valid with bitrate `1..14`, `h1` has a **different non-zero bitrate index** `1..15` (incl. the reserved `15`, which `h1` is never validated against) → expect `1` | [x] |
| 8 | `hdr_compare` | `h2` valid, `h1[0]` set to **every** value `0x00..0xFF` (B6: never read) and `h1[3]`/`h2[3]` random (B7) → expect `1` | [x] |
| 9 | `hdr_compare` | `h1` is an otherwise **completely invalid** header (sync `0x00`, reserved layer bits only where masked-out, reserved bitrate `15`) but agrees with valid `h2` under the `0xFE`/`0x0C`/free-format masks → expect `1` | [x] |
| 10 | `hdr_compare` | **aliased pointers**: `h1 == h2` pointing at the same valid buffer (C2) → expect `1`; and `h1 == h2` at an invalid buffer → expect `0` | [x] |
| 11 | `hdr_compare` | **overlapping** buffers: `h1 = buf+k`, `h2 = buf+k+1` etc. over a random byte pool, all `k` (C2) → outputs must match C | [x] |
| 12 | `hdr_compare` | **misaligned** pointers: same logical headers placed at offsets 0,1,2,3,5,7 in an allocation (C3) → expect identical results | [x] |
| 13 | `hdr_compare` | invalid-`h2` families with a *matching* `h1` (A1 wrong / A2 neither / A3 layer 0 / A5 bitrate 15 / A6 sample-rate 3), covering the full byte range of the offending field → expect `0` | [x] |
| 14 | `hdr_compare` | valid `h2`, `h1` mismatching under `0xFE` (B1) and/or `0x0C` (B3) and/or free-format (B4), full cross-product of the three mismatch flags (8 combinations) → expect `0` except the all-agree case | [x] |
| 15 | `hdr_compare` | **exhaustive** `h2[0] ∈ 0..255` × valid `h2[1..2]` × `h1 = h2` (A1 complete sweep) | [x] |
| 16 | `hdr_compare` | **exhaustive** `h2[1] ∈ 0..255` × `h2[2] ∈ 0..255` (65 536) with `h2[0]=0xff` and `h1 = h2` (A2,A3,A4,A5,A6,A7 complete sweep) | [x] |
| 17 | `hdr_compare` | **exhaustive** `h1[1] ∈ 0..255` × `h1[2] ∈ 0..255` (65 536) for each of 12 fixed representative `h2` values (valid & invalid) (B1..B5 complete sweep) | [x] |
| 18 | `hdr_compare` | **exhaustive** `h2[1] × h2[2] × h1[2]` (16 777 216) for several fixed `h1[1]`, and `h2[1] × h2[2] × h1[1]` for several fixed `h1[2]` — full 3-byte cross-products | [x] |
| 19 | `hdr_compare` | **randomized full sweep**: all 5 read-relevant bytes (`h1[1]`,`h1[2]`,`h2[0]`,`h2[1]`,`h2[2]`) plus unread bytes drawn from a seeded PRNG, ≥ 2 000 000 samples, biased so ~50 % of `h2` are valid headers | [x] |
| 20 | `hdr_compare` | **read-extent configurations** (C1): headers placed at the end of a mapped page with the following page unmapped, so that reading one byte too far faults — 1/2/3 mapped bytes, and `h1 = NULL` on every path where `hdr_valid(h2)` is false; run in a child process | [x] |
| 21 | `hdr_compare` | realistic **MP3 frame-header corpus**: every (version, layer, bitrate 0..14, sample-rate 0..2, padding, private, CRC) combination materialised as a real 4-byte header for `h2`, cross-multiplied with the same corpus for `h1` (sampled) → outputs must match C | [x] |
| 22 | `hdr_compare` | **COMPLETE input space**: all 2^32 combinations of `h1[1] × h1[2] × h2[1] × h2[2]` with `h2[0] = 0xff` (8-way sharded), plus all 2^24 of `h2[1] × h2[2] × h1[1]` for six `h2[0] != 0xff` values. Together with row 15 this is an *exhaustive* equivalence proof over every input the function can distinguish | [x] |

Row-to-test map:

| rows | test file |
|------|-----------|
| 1–14, 21 | `tests/valid_paths.rs` |
| 15–19, 22 | `tests/exhaustive.rs` |
| 20 | `tests/read_extent.rs` |

## Why row 22 makes the coverage complete

`hdr_compare` reads at most five bytes — `h2[0]`, `h2[1]`, `h2[2]`, `h1[1]`,
`h1[2]` — and `tests/read_extent.rs` *proves* (with guard pages) that no other
byte is ever dereferenced by either implementation. The function is therefore a
pure map from those five bytes to `int`. Row 22 enumerates the whole
`h2[0]==0xff` slice (2^32 points, the only slice where anything past the first
gate runs) and row 15 + row 22b sweep the `h2[0]!=0xff` slice. So the two
implementations are verified to agree on *every reachable input*, not merely on
a sample.

## Feature-combination matrix

`Cargo.toml` has **no `[features]` section**, so the power set of features is
`{∅}` — a single combination, which `--no-default-features` selects.
`c_src/CMakeLists.txt` has no `option()`/`if()`/`-D` switches, so there is a
single C configuration too. Both Cargo profiles are still exercised because the
`release` profile changes codegen (`panic = "abort"`).

| # | feature combination | profile | `cargo check` | `cargo build` | Phase B | Phase C | `nm -D` parity |
|---|--------------------|---------|---------------|---------------|---------|---------|----------------|
| 1 | `--no-default-features` (empty set) | `dev` | [x] | [x] | [x] 21/21 rows | [x] 16/16 rows | [x] |
| 2 | `--no-default-features --release` (empty set, `panic = "abort"`) | `release` | [x] | [x] | [x] 21/21 rows | [x] 16/16 rows | [x] |

Reproduce with `./verify.sh` (enumerates the feature power set from
`Cargo.toml`, builds the C `.so`, then runs `cargo check` + `cargo build` +
`cargo test --no-fail-fast` + an `nm -D` diff for every combination × profile).
Latest run: **37 passing tests, 0 failures, empty symbol diff, in both
profiles.**

## Test-suite validation (mutation testing)

Passing tests only mean something if they can fail. Fourteen mutants were
injected into `src/lib.rs`, rebuilt, and run against the suite; every one was
caught, and then the original source was restored (verified byte-identical):

| mutant injected into `src/lib.rs` | # tests that failed |
|-----------------------------------|---------------------|
| `h1[1]^h2[1]` mask `0xFE` → `0xFF` | 15 |
| reserved bitrate `15` → `14` | 25 |
| reserved sample-rate `3` → `2` | 26 |
| sync alternative `0xE2` → `0xE0` | 22 |
| free-format `^` → `&` | 15 |
| sample-rate mask `0x0C` → `0x0E` | 16 |
| layer-reserved check deleted | 15 |
| `h[0] == 0xff` → `== 0xfe` | 27 |
| return `1` → return `2` | 27 |
| eager read of `h1[1]` before `hdr_valid(h2)` | 1 (`read_extent_matches_c`) |
| eager read of `h[2]` before the `h[1]` gates | 1 (`read_extent_matches_c`) |
| extra read of `h1[3]` | 1 (`read_extent_matches_c`) |
| extra read of `h1[0]` | 1 (`read_extent_matches_c`) |
| read `h1[2]` before the `h1[1]` gate | 1 (`read_extent_matches_c`) |

Mutant survivors: **0**.
