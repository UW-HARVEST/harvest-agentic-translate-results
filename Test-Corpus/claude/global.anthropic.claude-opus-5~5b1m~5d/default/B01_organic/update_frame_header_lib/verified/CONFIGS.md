# CONFIGS.md — Phase A: configuration surface table

Derived mechanically from the `switch` / `case` / `if` / ternary branches in
`c_src/src/lib.c` and the public type in `c_src/include/lib.h`.

## Public entry points (complete)

| entry point | signature | level |
|-------------|-----------|-------|
| `update_frame_header` | `void update_frame_header(tflac *t)` | the **only** one — it is simultaneously the highest- and lowest-level public function; there is no convenience wrapper to hide behind |

There is no init/open/close, no option-setter function and no `#ifdef`. All
"runtime options" are therefore **struct fields the caller writes before the
call**; they are the configuration axes below. The three input fields of the
`tflac` record are `samplerate`, `channels`, `bitdepth`, `channel_mode`,
`cur_blocksize`; `frame_header` is the output (and is unconditionally
overwritten at `lib.c:12`, so its input value is a no-op axis — still tested).

## Configuration axes the C actually branches on

### Axis BS — `cur_blocksize` (`switch` at `lib.c:13`, 13 cases + ternary default)

| class | condition | blocksize nibble (bits 12..15) |
|-------|-----------|-------------------------------|
| BS1..BS13 | `192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768` | `0x1,0x2,0x3,0x4,0x5,0x8,0x9,0xA,0xB,0xC,0xD,0xE,0xF` |
| BS14 | default **and** `<= 256` | `0x6` |
| BS15 | default **and** `> 256` | `0x7` |

### Axis SR — `samplerate` (`switch` at `lib.c:58`, 11 cases + nested-`if` default)

| class | condition | samplerate nibble (bits 8..11) |
|-------|-----------|-------------------------------|
| SR1..SR11 | `882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000` | `0x1..0xB` |
| SR12 | default, `%1000==0`, `/1000 < 256` | `0xC` |
| SR13 | default, `%1000==0`, `/1000 >= 256` | none |
| SR14 | default, `%1000!=0`, `< 65536` | `0xD` |
| SR15 | default, `%1000!=0`, `>= 65536`, `%10==0`, `/10 < 65536` | `0xE` |
| SR16 | default, `%1000!=0`, `>= 65536`, `%10==0`, `/10 >= 65536` | none |
| SR17 | default, `%1000!=0`, `>= 65536`, `%10!=0` | none |

### Axis CM — `channel_mode` (`% 4` then `switch` at `lib.c:107`)

| class | `channel_mode % 4` | enum | effect |
|-------|--------------------|------|--------|
| CM0 | 0 | `TFLAC_CHANNEL_INDEPENDENT` | `\|= (channels - 1) << 4` (data-dependent!) |
| CM1 | 1 | `TFLAC_CHANNEL_LEFT_SIDE` | `\|= 0x8 << 4` |
| CM2 | 2 | `TFLAC_CHANNEL_SIDE_RIGHT` | `\|= 0x9 << 4` |
| CM3 | 3 | `TFLAC_CHANNEL_MID_SIDE` | `\|= 0xA << 4` |

Every one of the 256 `u8` values is a *valid* configuration because of the `% 4`.
CM0 is the only class where `channels` is read at all.

### Axis CH — `channels` (only observable under CM0)

| class | condition | note |
|-------|-----------|------|
| CH0 | `0` | unsigned underflow (see ERRORS.md #12) |
| CH1 | `1..=8` | legal FLAC counts, `(ch-1)<<4` fits the nibble |
| CH2 | `9..=16` | still fits bits 4..7 (`15<<4 == 0xF0`) |
| CH3 | `17..` | spills out of the nibble into the samplerate/blocksize fields |
| CH4 | `>= 0x1000_0001` | `<< 4` truncates the top bits away |

### Axis BD — `bitdepth` (`switch` at `lib.c:123`, 6 cases + default)

| class | condition | sample-size field (bits 1..3) |
|-------|-----------|------------------------------|
| BD1..BD6 | `8, 12, 16, 20, 24, 32` | `1,2,4,5,6,7` |
| BD7 | anything else | none |

### Axis FH — incoming `frame_header`

| class | condition |
|-------|-----------|
| FH0 | `0` |
| FH1 | `0xFFFF_FFFF` / random — must be fully overwritten, never ORed into |

## Configuration table (cross-product, pruned to what the C distinguishes)

Each row is exercised with **many randomized inputs** (fixed-seed
`SplitMix64`, seed `0x5DEE_CE66_D1CE_F00D`): the axis under test is pinned to
the class and every other field is randomized over the classes / value ranges
that the C treats differently. Rows are checked off only after the whole
randomized batch matches byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `update_frame_header` | BS1..BS13 (each of the 13 exact `case` blocksizes) × randomized SR/CM/CH/BD/FH | [x] |
| 2 | `update_frame_header` | BS14 (default, `<=256`) — randomized blocksizes in `0..=256` minus the cases, incl. `0`, `1`, `255`, `256`-neighbours × randomized rest | [x] |
| 3 | `update_frame_header` | BS15 (default, `>256`) — randomized blocksizes in `257..=u32::MAX` minus the cases × randomized rest | [x] |
| 4 | `update_frame_header` | BS exhaustive sweep `cur_blocksize = 0..=70_000` (covers every case, both default arms and all boundaries) with 3 fixed field profiles | [x] |
| 5 | `update_frame_header` | SR1..SR11 (each of the 11 exact `case` samplerates) × randomized BS/CM/CH/BD/FH | [x] |
| 6 | `update_frame_header` | SR12 (`%1000==0`, `/1000<256`): randomized `1000*k`, `k ∈ 0..256`, minus the cases × randomized rest | [x] |
| 7 | `update_frame_header` | SR13 (`%1000==0`, `/1000>=256`): randomized `1000*k`, `k ∈ 256..=4_294_967` × randomized rest | [x] |
| 8 | `update_frame_header` | SR14 (`%1000!=0`, `<65536`): randomized `0..65536` non-multiples-of-1000 × randomized rest | [x] |
| 9 | `update_frame_header` | SR15 (`%1000!=0`, `>=65536`, `%10==0`, `/10<65536`): randomized `10*k` in `65536..655_360` with `%1000!=0` × randomized rest | [x] |
| 10 | `update_frame_header` | SR16 (`%1000!=0`, `>=65536`, `%10==0`, `/10>=65536`): randomized `10*k >= 655_360`, `%1000!=0` × randomized rest | [x] |
| 11 | `update_frame_header` | SR17 (`%1000!=0`, `>=65536`, `%10!=0`): randomized × randomized rest | [x] |
| 12 | `update_frame_header` | SR exhaustive sweep `samplerate = 0..=200_000` (all 11 cases + SR12/13/14/15 + the 65536 boundary) with 3 fixed field profiles | [x] |
| 13 | `update_frame_header` | SR exhaustive sweep over the decahertz band `samplerate = 650_000..=660_000` (SR15↔SR16 crossover at 655_360) | [x] |
| 14 | `update_frame_header` | CM0 × CH1 (`channels = 1..=8`, the legal FLAC counts) × randomized BS/SR/BD/FH | [x] |
| 15 | `update_frame_header` | CM0 × CH0 (`channels = 0`, underflow) × randomized BS/SR/BD/FH | [x] |
| 16 | `update_frame_header` | CM0 × CH2 (`channels = 9..=16`) × randomized rest | [x] |
| 17 | `update_frame_header` | CM0 × CH3/CH4 (`channels >= 17`, incl. `0x0FFF_FFFF`, `0x1000_0000`, `0x1000_0001`, `u32::MAX`) × randomized rest | [x] |
| 18 | `update_frame_header` | CM1 / CM2 / CM3 (`channel_mode % 4 ∈ {1,2,3}`) × randomized `channels` (must be ignored) × randomized rest | [x] |
| 19 | `update_frame_header` | CM exhaustive: all 256 `channel_mode` byte values × randomized rest (proves the `% 4` aliasing incl. `TFLAC_CHANNEL_MODE_COUNT` and `255`) | [x] |
| 20 | `update_frame_header` | BD1..BD6 (each of the 6 exact `case` bitdepths) × randomized rest | [x] |
| 21 | `update_frame_header` | BD7 exhaustive `bitdepth = 0..=64` plus randomized `65..=u32::MAX` × randomized rest | [x] |
| 22 | `update_frame_header` | FH0/FH1: incoming `frame_header ∈ {0, 0xFFFF_FFFF, random}` — output must not depend on it × randomized rest | [x] |
| 23 | `update_frame_header` | **realistic full pipeline**: the cross-product of the 13 `case` blocksizes × the 11 `case` samplerates × 4 modes × `channels 1..=8` × the 6 `case` bitdepths (all "well-formed encoder" configs, enumerated exhaustively) | [x] |
| 24 | `update_frame_header` | **interaction**: CM0 × `channels >= 17` (nibble overflow) × every SR class (proves the corrupted samplerate nibble matches) | [x] |
| 25 | `update_frame_header` | **interaction**: CM0 × `channels == 0` (`0xFFFF_FFF0`) × every BS and BD class (proves the OR-saturated result matches) | [x] |
| 26 | `update_frame_header` | **unconstrained fuzz**: all 5 input fields drawn uniformly from the full `u32`/`u8` domain, 2_000_000 iterations | [x] |
| 27 | `update_frame_header` | **structure-aware fuzz**: each field drawn from a per-axis "interesting value" pool (case values ± 1, powers of two, boundaries, extremes), 2_000_000 iterations | [x] |
| 28 | `update_frame_header` | repeated / in-place invocation: call both libs twice on the same record (idempotence of the unconditional assignment at `lib.c:12`), and alias-safety of a record placed at a 4-byte-but-not-8-byte-aligned address | [x] |
| 29 | `update_frame_header` | **exhaustive SR axis**: every `samplerate` in `0..=4_300_000` — covers all 11 cases and every default sub-branch, both `/1000` and `/10` thresholds, and ~4300 full periods of the `%1000` / `%10` classification | [x] |
| 30 | `update_frame_header` | **exhaustive BS axis**: every `cur_blocksize` in `0..=1_048_576` (all 13 cases, both ternary arms, every power-of-two neighbourhood in range) × 2 profiles | [x] |
| 31 | `update_frame_header` | **exhaustive CH axis under CM0**: every `channels` in `0..=1_048_576` plus the `<< 4` truncation neighbourhoods around `2^28` and `2^32` | [x] |
| 32 | `update_frame_header` | **exhaustive BD axis**: every `bitdepth` in `0..=65_536` × 2 profiles | [x] |
| 33 | `update_frame_header` | **complete class cross-product**: the FULL cartesian product of class representatives on all five axes (20 blocksizes × 23 samplerates × 13 channel counts × 9 channel modes × 10 bitdepths = 538 200 configurations), so every interaction of every BS/SR/CM/CH/BD class is exercised at least once | [x] |

All 33 rows are implemented in `tests/phase_b_configs.rs` and pass.

## Why this is effectively exhaustive

Rows 29-32 sweep each axis exhaustively over the whole region where the C's
classification actually varies, and beyond that region the classification is
constant, so the sweeps plus the full-domain randomized rows (26/27) cover every
reachable code path:

* `cur_blocksize`: exact-match against 13 constants (max `32768`) plus the
  `<= 256` ternary. Every value above `1_048_576` is `> 256` and matches no
  case, so it deterministically yields `0x7`.
* `samplerate`: every threshold in the default arm (`/1000 vs 256` → 256 000,
  `< 65536`, `/10 vs 65536` → 655 360) and the largest `case` (882 000) lie below
  4 300 000. Every value above 655 360 falls into one of the three
  "contributes-no-bits" classes.
* `bitdepth`: exact-match against 6 constants (max `32`); everything above 32 is
  `default`.
* `channels`: `(channels - 1) << 4` is a pure wrapping arithmetic expression,
  swept exhaustively over `0..=1_048_576` and over both truncation
  neighbourhoods (`2^28` and `2^32`).
* `channel_mode`: all 256 possible byte values are enumerated exhaustively.

Row 33 then verifies that the five independently-verified contributions compose
correctly (the C combines them with `|=`, so interactions such as the
`channels`-overflow bleeding into the samplerate nibble are real and covered).
