# CONFIGS.md — configuration-surface table (valid inputs)

Axes derived mechanically from every `if` / `while` / arithmetic branch in
`c_src/src/lib.c` and every field of `struct tflac` in `c_src/include/lib.h`.
There are no `#ifdef`s, no runtime option setters, and no `switch` statements —
the entire configuration surface is (a) the argument of `tflac_size_memory` and
(b) the ten input fields of `struct tflac`.

## Axes the C actually branches on

| axis | states the C distinguishes |
|------|----------------------------|
| `blocksize` (`flac_validate`) | `<16` reject · `16` · interior · `65535` · `>65535` reject; **and** its 2-adic valuation (how many times it is divisible by 2) which drives the `partition_order` loop |
| `blocksize` (`tflac_size_memory`) | no rejection; `blocksize*4` non-wrapping vs wrapping (`>= 2^30`); the `& 0xFFFFFFF0` mask; the `5*` wrapping multiply |
| `samplerate` | `0` reject · `1` · interior · `655350` · `>655350` reject |
| `channels` | `0` reject · `==2` (side-stereo eligible) · `1`,`3..8` (not eligible) · `>8` reject |
| `bitdepth` | `0` reject · `1..16` (rice default 14) · `17..31` (rice default 30) · `==32` (rice 30 **and** forces independent) · `>32` reject |
| `channel_mode` | `==0` untouched · `1..3` valid non-independent · `4..255` out-of-enum non-independent (treated identically to 1..3) |
| `max_rice_value` | `==0` → defaulted from `bitdepth` · `1..30` → kept · `31..255` reject |
| `max_partition_order` | `0..15` accepted · `16..255` reject |
| `min_partition_order` | `<= max` accepted · `> max` reject |
| `partition_order` (output) | loop runs 0 times / some / until it hits `max_partition_order` |
| `cur_blocksize` (output) | always overwritten with `blocksize` on success only |

## Rows — one per meaningful combination

Every row is driven with **many randomised inputs** (fixed seed, deterministic
xorshift PRNG in `tests/differential.rs`), and asserts the return value **and**
all 28 struct bytes match byte-for-byte between the C `.so` and the Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `tflac_size_memory` | `blocksize` in `0..=65535` (no wrap): exhaustive | [x] |
| 2  | `tflac_size_memory` | `blocksize` on masking/alignment boundaries: every `b` where `15+4b` is `≡0 mod 16` and its neighbours, plus `0,1,2,3,4,15,16,17` | [x] |
| 3  | `tflac_size_memory` | `blocksize >= 2^30` so `blocksize*4` wraps, incl. `0x3FFFFFFF..0xFFFFFFFF` and randomised full-range `u32` | [x] |
| 4  | `flac_validate` | fully randomised full-range fields (all 28 bytes random, mostly rejected) — smoke fuzz over the whole surface | [x] |
| 5  | `flac_validate` | valid + `channel_mode == 0` (independent), `channels` random `1..=8`, `bitdepth` random `1..=32` | [x] |
| 6  | `flac_validate` | valid + `channel_mode ∈ 1..=3`, `channels == 2`, `bitdepth ∈ 1..=31` → mode **preserved** | [x] |
| 7  | `flac_validate` | valid + `channel_mode ∈ 1..=3`, `channels == 2`, `bitdepth == 32` → mode **forced to 0** | [x] |
| 8  | `flac_validate` | valid + `channel_mode ∈ 1..=3`, `channels ∈ {1,3,4,5,6,7,8}` → mode **forced to 0** | [x] |
| 9  | `flac_validate` | valid + `channel_mode ∈ 4..=255` (out-of-enum), `channels == 2`, `bitdepth ∈ 1..=31` → mode **preserved verbatim** | [x] |
| 10 | `flac_validate` | valid + `channel_mode ∈ 4..=255` (out-of-enum), `channels != 2` or `bitdepth == 32` → mode **forced to 0** | [x] |
| 11 | `flac_validate` | valid + `max_rice_value == 0`, `bitdepth ∈ 1..=16` → defaults to 14 | [x] |
| 12 | `flac_validate` | valid + `max_rice_value == 0`, `bitdepth ∈ 17..=32` → defaults to 30 | [x] |
| 13 | `flac_validate` | valid + `max_rice_value ∈ 1..=30` → kept unchanged (incl. boundaries 1 and 30) | [x] |
| 14 | `flac_validate` | valid + `min_partition_order == max_partition_order` (`0..=15`) → loop never advances | [x] |
| 15 | `flac_validate` | valid + `min=0`, `max=15`, `blocksize` odd → loop exits immediately, `partition_order == 0` | [x] |
| 16 | `flac_validate` | valid + `min=0`, `max=15`, `blocksize = 32768` (2-adic valuation 15) → `partition_order` climbs to `max` (shift reaches `1<<16`) | [x] |
| 17 | `flac_validate` | valid + `min=0`, `max=15`, `blocksize = odd * 2^k` for every `k ∈ 0..=15` → loop stops at `min(k, max)` | [x] |
| 18 | `flac_validate` | valid + `min`/`max` random with `min <= max`, `blocksize` random `16..=65535` → loop clamped by `max` | [x] |
| 19 | `flac_validate` | boundary-value sweep: `blocksize ∈ {16,17,65534,65535}` × `samplerate ∈ {1,2,655349,655350}` × `channels ∈ 1..=8` × `bitdepth ∈ {1,16,17,31,32}` | [x] |
| 20 | `flac_validate` | output fields `partition_order` / `cur_blocksize` **pre-seeded with random garbage** on both success and reject paths → confirms identical overwrite / non-overwrite | [x] |
| 21 | `flac_validate` | called twice in a row on the same struct (idempotence / second-pass over already-normalised state, e.g. `max_rice_value` now non-zero) | [x] |
| 22 | `flac_validate` + `tflac_size_memory` | composed pipeline: validate, then feed the resulting `cur_blocksize` into `tflac_size_memory` | [x] |

## Not applicable

* No binary / driver executable is built — `c_src/CMakeLists.txt` declares only
  `add_library(... SHARED src/lib.c)`, so there is no stdout to compare.
* `translation/Cargo.toml` declares **no `[features]` table**, so the only
  feature combination is the default (empty) one. Verified by
  `cargo check --no-default-features` and `cargo test --no-default-features`
  both succeeding with identical results.

## How to reproduce

```
cd translation && ./verify.sh
```

`verify.sh` builds the C `.so`, builds the Rust cdylib, enumerates the feature
combinations from `Cargo.toml`, runs the test suite for each, and diffs `nm -D`.

### Why the explicit build step matters

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]`-only lib target.
An earlier run of this suite therefore loaded a *stale* `.so` and every
differential test passed vacuously — verified by mutating `src/lib.rs` and
observing zero failures. Two fixes are in place:

1. `verify.sh` runs `cargo build --release` before `cargo test`.
2. `tests/harness/mod.rs::assert_so_fresh` hard-fails the tests if
   `libflac_validate_lib.so` is older than any file under `src/` or
   `Cargo.toml`, so the failure mode can never be silent again.

### Harness non-vacuity (mutation testing)

Nineteen single-token mutations were injected into `translation/src/lib.rs`, the
suite was re-run through `verify.sh`, and the source was restored bit-identically
afterwards. **All 19 were caught** (each produced 3–19 failing tests):

| mutation | detected |
|----------|----------|
| `tflac_size_memory`: constant `15` → `16` | yes (4) |
| `tflac_size_memory`: mask `0xFFFFFFF0` → `0xFFFFFFF8` | yes (4) |
| `tflac_size_memory`: multiplier `5` → `6` | yes (4) |
| `blocksize < 16` → `<= 16` | yes (14) |
| `samplerate > 655350` → `>=` | yes (5) |
| `channels > 8` → `> 7` | yes (14) |
| `bitdepth > 32` → `> 31` | yes (14) |
| `channel_mode` guard `||` → `&&` | yes (14) |
| `channel_mode` forced to `LEFT_SIDE` instead of `INDEPENDENT` | yes (14) |
| rice-default split `bitdepth <= 16` → `< 16` | yes (16) |
| rice default `14` → `13` | yes (16) |
| rice default `30` → `29` | yes (15) |
| `max_rice_value > 30` → `>= 30` | yes (14) |
| `max_partition_order > 15` → `> 14` | yes (19) |
| `min > max` → `min >= max` | yes (17) |
| partition-order loop `<` → `<=` | yes (15) |
| shift amount `partition_order + 1` → `partition_order` | yes (17) |
| `cur_blocksize` written on error paths too | yes (3) |
| `cur_blocksize` reordered before the loop | yes |
