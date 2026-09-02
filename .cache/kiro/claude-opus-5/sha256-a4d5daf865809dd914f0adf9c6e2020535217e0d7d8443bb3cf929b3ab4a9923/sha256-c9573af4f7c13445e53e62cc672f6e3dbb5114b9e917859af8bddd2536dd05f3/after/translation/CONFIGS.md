# CONFIGS.md — Phase B configuration-surface table

## Public entry points

The whole public API, from `c_src/include/lib.h`:

| entry point | kind |
|-------------|------|
| `void update_frame_header(tflac *t)` | the **only** exported function — it is simultaneously the lowest-level and the highest-level entry point; there is no convenience wrapper to hide behind |

There is no init/open/close, no allocator, no opaque handle. "Setting up state"
means writing the input fields of `struct tflac` directly; the caller owns the
struct. So the configuration surface is entirely the **input field space**.

## Runtime options / modes

Grepped from the `switch` / `if` statements in `c_src/src/lib.c`:

| axis | field | values the C actually distinguishes |
|------|-------|--------------------------------------|
| **A** block size | `cur_blocksize` (`u32`) | 15 classes: the 13 enumerated cases `192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768`, plus `default && <= 256`, plus `default && > 256` |
| **B** sample rate | `samplerate` (`u32`) | 17 classes: the 11 enumerated cases `882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000`, plus 6 `default` sub-branches (see below) |
| **C** channel mode | `channel_mode` (`u8`), reduced by `% 4` | 4 classes: `0` independent, `1` left/side, `2` side/right, `3` mid/side. Mode 0 additionally reads **`channels`**, so mode 0 is itself a sub-axis over `channels` |
| **C'** channels | `channels` (`u32`) | only read when `channel_mode % 4 == 0`: `0` (underflow), `1..=8` (the FLAC-valid range, 8 distinct nibbles), `> 8` (spills out of the 4-bit field), `u32::MAX` (shifts bits off the top) |
| **D** bit depth | `bitdepth` (`u32`) | 7 classes: `8, 12, 16, 20, 24, 32`, plus `default` (no bits) |

`samplerate` `default` sub-branches (nested `if`/`else if` chain):

| B-default sub-class | condition | nibble |
|---------------------|-----------|--------|
| B-d1 | `%1000 == 0 && /1000 < 256` | `0x0C` |
| B-d2 | `%1000 == 0 && /1000 >= 256` | none |
| B-d3 | `%1000 != 0 && < 65536` | `0x0D` |
| B-d4 | `%1000 != 0 && >= 65536 && %10 == 0 && /10 < 65536` | `0x0E` |
| B-d5 | `%1000 != 0 && >= 65536 && %10 == 0 && /10 >= 65536` | none |
| B-d6 | `%1000 != 0 && >= 65536 && %10 != 0` | none |

No `#ifdef`, no compile-time option, no global state, no byte-order or
element-type axis (the only "format" is the fixed-layout `struct tflac`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the feature power-set is `{default} == {}` — one configuration.
Verified by grep; Phase D re-runs the suite under `--no-default-features`
anyway, which is the same code path.

## Combination rows

The four axes are independent (each writes a disjoint bit field of
`frame_header`), *except* that A/C'/D bits can collide with the C' spill when
`channels` is out of range — which is exactly the interaction worth crossing.
The full cross product is 15 × 17 × (3 + 11) × 7 = 24 990 cells; the rows below
prune that to the combinations the code actually distinguishes, and each row is
driven with **many randomized inputs** (fixed seed) that sweep the other axes,
so the cross product is covered stochastically on top of the structured rows.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `update_frame_header` | A = each of the 13 enumerated `cur_blocksize` values × randomized B/C/C'/D | [x] |
| 2 | `update_frame_header` | A = `default && <= 256` (randomized values in `0..=256` minus enumerated) × randomized B/C/C'/D | [x] |
| 3 | `update_frame_header` | A = `default && > 256` (randomized values `257..=u32::MAX` minus enumerated) × randomized B/C/C'/D | [x] |
| 4 | `update_frame_header` | B = each of the 11 enumerated `samplerate` values × randomized A/C/C'/D | [x] |
| 5 | `update_frame_header` | B = B-d1 (`%1000==0`, `/1000<256`) randomized × randomized A/C/C'/D | [x] |
| 6 | `update_frame_header` | B = B-d2 (`%1000==0`, `/1000>=256`) randomized × randomized A/C/C'/D | [x] |
| 7 | `update_frame_header` | B = B-d3 (`%1000!=0`, `<65536`) randomized × randomized A/C/C'/D | [x] |
| 8 | `update_frame_header` | B = B-d4 (`%1000!=0`, `>=65536`, `%10==0`, `/10<65536`) randomized × randomized A/C/C'/D | [x] |
| 9 | `update_frame_header` | B = B-d5 (`%1000!=0`, `>=65536`, `%10==0`, `/10>=65536`) randomized × randomized A/C/C'/D | [x] |
| 10 | `update_frame_header` | B = B-d6 (`%1000!=0`, `>=65536`, `%10!=0`) randomized × randomized A/C/C'/D | [x] |
| 11 | `update_frame_header` | C = independent (`channel_mode % 4 == 0`) × C' = `channels` in `1..=8` × randomized A/B/D | [x] |
| 12 | `update_frame_header` | C = independent × C' = `channels == 0` (underflow) × randomized A/B/D — interaction: the `0xFFFFFFF0` spill overwrites the A and D fields | [x] |
| 13 | `update_frame_header` | C = independent × C' = `channels` in `9..=255` (spill past the 4-bit field) × randomized A/B/D | [x] |
| 14 | `update_frame_header` | C = independent × C' = `channels` randomized over the whole `u32` range (incl. `u32::MAX`, shift-off-the-top) × randomized A/B/D | [x] |
| 15 | `update_frame_header` | C = left/side (`channel_mode % 4 == 1`, i.e. `channel_mode ∈ {1,5,...,253}`) × randomized `channels` (must be ignored) × randomized A/B/D | [x] |
| 16 | `update_frame_header` | C = side/right (`% 4 == 2`) × randomized `channels` × randomized A/B/D | [x] |
| 17 | `update_frame_header` | C = mid/side (`% 4 == 3`) × randomized `channels` × randomized A/B/D | [x] |
| 18 | `update_frame_header` | C = every one of the 256 possible `channel_mode` byte values × randomized A/B/C'/D | [x] |
| 19 | `update_frame_header` | D = each of `8, 12, 16, 20, 24, 32` × randomized A/B/C/C' | [x] |
| 20 | `update_frame_header` | D = `default` (randomized `bitdepth` avoiding the 6 valid values, incl. 0 and `u32::MAX`) × randomized A/B/C/C' | [x] |
| 21 | `update_frame_header` | **all-realistic combination**: A ∈ enumerated, B ∈ enumerated, C ∈ 0..=3 with C' ∈ 1..=8, D ∈ valid — the full realistic cross product driven exhaustively (13 × 11 × 4 × 8 × 6) | [x] |
| 22 | `update_frame_header` | **unconstrained fuzz**: all six struct fields drawn uniformly at random over their full integer ranges (incl. the pre-existing `frame_header` value, which the C must overwrite, never OR into) | [x] |
| 23 | `update_frame_header` | **aliasing / repeat-call shape**: the same struct passed to the same `.so` twice in a row, and to C then Rust, to confirm the function is idempotent and does not read the incoming `frame_header` | [x] |
| 24 | `update_frame_header` | **boundary sweep**: every value in `0..=1024` and each of `{n-1, n, n+1}` around all 13 A constants, 11 B constants and 6 D constants, applied to each axis in turn | [x] |

## Result

All 24 rows pass, in both the release and debug profiles and under every feature
configuration (`--all-features`, default, `--no-default-features` — the crate has
no `[features]` table, so that is the whole power set). No divergence was found
on any valid-path row.

Coverage beyond the table, in `tests/phase_d_parity.rs`:

| test | coverage |
|------|----------|
| `d03_exhaustive_low_million_per_axis` | every value `0..=1_000_000` on each of the four `u32` axes |
| `d04_full_u32_stride_per_axis` | whole `u32` domain per axis by prime stride 9973 (coprime with 1000 and 10, so it lands in all six `samplerate` sub-branches) |
| `d05_channel_mode_exhaustive_cross` | all 256 `channel_mode` bytes × 8 blocksizes × 8 samplerates × 7 channel counts × 6 bitdepths |
| `d06`–`d09` (`--ignored`) | **exhaustive over all 2^32 values** of `samplerate`, `cur_blocksize`, `channels` and `bitdepth` respectively |

`d06`–`d09` together make ~17.2 billion differential comparisons and report zero
divergences, which reduces the per-axis claim from "sampled" to "exhaustive".
They are `#[ignore]`d because they take ~64 s wall clock; run them with:

```
cargo test --release --test phase_d_parity -- --ignored --nocapture
```
