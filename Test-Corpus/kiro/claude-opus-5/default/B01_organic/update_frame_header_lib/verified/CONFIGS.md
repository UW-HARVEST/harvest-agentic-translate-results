# CONFIGS.md — configuration-surface table (Phase A, gates Phase B)

## Mechanical derivation of the axes

The whole public API is one entry point, `void update_frame_header(tflac *t)`.
It is also the *lowest-level* entry point — there is no convenience wrapper and
no other non-static function in the library, so "exercise the low-level entry
points, not just the wrappers" collapses to: drive `update_frame_header`
directly over its full input space.

"Options" are not function arguments here; they are the **struct fields the C
branches on**. Enumerated from the four `switch` statements and the nested
`if`/`else if` chain in `c_src/src/lib.c`:

* **Axis BS — `cur_blocksize`** (first `switch`, bits 12–15). 13 explicit
  `case`s + a 2-way `default` (`<= 256` vs `> 256`) ⇒ **15 classes**:
  `192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
  default≤256, default>256`.
* **Axis SR — `samplerate`** (second `switch`, bits 8–11). 11 explicit `case`s
  + 6 `default` sub-classes ⇒ **17 classes**:
  `882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000,
  96000`, then `%1000==0 && /1000<256`, `%1000==0 && /1000>=256` (silent),
  `%1000!=0 && <65536`, `%1000!=0 && >=65536 && %10==0 && /10<65536`,
  `%1000!=0 && >=65536 && %10==0 && /10>=65536` (silent),
  `%1000!=0 && >=65536 && %10!=0` (silent).
* **Axis CM — `channel_mode % 4`** (third `switch`, bits 4–7) ⇒ **4 classes**
  `0/1/2/3`; plus the *unfolded* input range `channel_mode ∈ 0..=255` which the
  `% 4` maps onto them (so `4`, `5`, `255` are real inputs, not errors).
* **Axis CH — `channels`**, live **only when `CM == 0`**, via
  `(channels - 1) << 4` ⇒ **5 shapes**: `0` (unsigned underflow),
  `1` (empty/one), `2..=8` (typical FLAC), `9..=16` (fills the nibble),
  `> 16` (bleeds into neighbouring fields), and `>= 0xF0000000` (`<< 4`
  truncates mod 2^32).
* **Axis BD — `bitdepth`** (fourth `switch`, bits 1–3). 6 explicit `case`s +
  `default` (silent) ⇒ **7 classes**: `8, 12, 16, 20, 24, 32, default`.
* **Axis IN — initial `frame_header` value.** The C *assigns* (`=`) the sync
  word first and only then ORs, so a pre-existing `frame_header` must be fully
  discarded. Two shapes: `0` and `0xFFFFFFFF` (pre-dirtied).
* **Axis PAD — struct tail padding** (bytes 13–15, from `tflac_u8
  channel_mode` at offset 12 followed by `tflac_u32` at 16): must be preserved
  untouched. Two shapes: zeroed and `0xAA`-filled.
* `#ifdef` axes: **none** in the C source. `[features]` in
  `translation/Cargo.toml`: **none**. No `[[bin]]` driver ⇒ no stdout compare.

Full cross-product is 15×17×4(+CH 6)×7×2×2. The table below is that product
**pruned to the combinations the code actually distinguishes**: each row pins
the axis it targets and *randomizes every other axis* over its full class set
(fixed-seed xorshift PRNG, `SEED = 0x5EED_1234_ABCD_F00D`, 4000 random draws
per row unless noted), so every row is itself a cross-product sample rather
than one hand-picked value. Row 0 is the exhaustive whole-space sweep.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 0 | `update_frame_header` | **Full-axis random sweep**: all of BS×SR×CM×CH×BD×IN×PAD drawn from their class sets, 200 000 draws, seeded | [x] |
| 1 | `update_frame_header` | BS = each of the 13 exact `case` block sizes (192/576/1152/2304/4608/256/512/1024/2048/4096/8192/16384/32768); SR/CM/CH/BD/IN/PAD random | [x] |
| 2 | `update_frame_header` | BS = `default` branch, `cur_blocksize <= 256` (random over `0..=256` minus 192,256); other axes random | [x] |
| 3 | `update_frame_header` | BS = `default` branch, `cur_blocksize > 256` (random over `257..=u32::MAX` minus the exact cases); other axes random | [x] |
| 4 | `update_frame_header` | SR = each of the 11 exact `case` sample rates (882000/176400/192000/8000/16000/22050/24000/32000/44100/48000/96000); other axes random | [x] |
| 5 | `update_frame_header` | SR = `default`, `%1000==0 && /1000<256` → `0x0C` (random `k*1000`, `k<256`, incl. `0`); other axes random | [x] |
| 6 | `update_frame_header` | SR = `default`, `%1000==0 && /1000>=256` → **no SR bits** (random `k*1000`, `256<=k`, no `u32` overflow); other axes random | [x] |
| 7 | `update_frame_header` | SR = `default`, `%1000!=0 && <65536` → `0x0D`; other axes random | [x] |
| 8 | `update_frame_header` | SR = `default`, `%1000!=0 && >=65536 && %10==0 && /10<65536` → `0x0E`; other axes random | [x] |
| 9 | `update_frame_header` | SR = `default`, `%1000!=0 && >=65536 && %10==0 && /10>=65536` → **no SR bits**; other axes random | [x] |
| 10 | `update_frame_header` | SR = `default`, `%1000!=0 && >=65536 && %10!=0` → **no SR bits**; other axes random | [x] |
| 11 | `update_frame_header` | CM: `channel_mode % 4 == 0` (INDEPENDENT) × CH = `1` and CH = `2..=8`; other axes random | [x] |
| 12 | `update_frame_header` | CM ≡ 0 × CH = `0` (underflow to `0xFFFFFFF0`); other axes random | [x] |
| 13 | `update_frame_header` | CM ≡ 0 × CH = `9..=16` (fills the 4-bit nibble exactly); other axes random | [x] |
| 14 | `update_frame_header` | CM ≡ 0 × CH = `17..=0xEFFFFFFF` (bits bleed into SR/BS/sync nibbles); other axes random | [x] |
| 15 | `update_frame_header` | CM ≡ 0 × CH >= `0xF0000000` (`<< 4` truncates mod 2^32); other axes random | [x] |
| 16 | `update_frame_header` | CM ≡ 1 (LEFT_SIDE) — `channel_mode ∈ {1,5,9,…,253}` random; `channels` random *and must be ignored*; other axes random | [x] |
| 17 | `update_frame_header` | CM ≡ 2 (SIDE_RIGHT) — `channel_mode ∈ {2,6,…,254}` random; `channels` ignored; other axes random | [x] |
| 18 | `update_frame_header` | CM ≡ 3 (MID_SIDE) — `channel_mode ∈ {3,7,…,255}` random; `channels` ignored; other axes random | [x] |
| 19 | `update_frame_header` | CM: full `channel_mode` sweep `0..=255` exhaustive × random other axes (covers `4 == TFLAC_CHANNEL_MODE_COUNT` and every out-of-enum byte) | [x] |
| 20 | `update_frame_header` | BD = each of `8, 12, 16, 20, 24, 32`; other axes random | [x] |
| 21 | `update_frame_header` | BD = `default` (random from `{0,1,…}` excluding the 6 cases, incl. `u32::MAX`); other axes random | [x] |
| 22 | `update_frame_header` | IN: `frame_header` pre-dirtied to random non-zero garbage (must be overwritten, not ORed into); other axes random | [x] |
| 23 | `update_frame_header` | PAD: tail padding bytes 13–15 pre-filled `0xAA`/random; assert all 24 struct bytes match afterwards; other axes random | [x] |
| 24 | `update_frame_header` | Aliasing / repeated invocation: call the same struct **twice in a row** on both libs (idempotence of the `=` then `|=` sequence); other axes random | [x] |
| 25 | `update_frame_header` | Interaction focus: BS exact-case × SR exact-case × CM ≡ 0 × CH `>16` — the combination where an over-large channel count corrupts an otherwise perfectly-encoded header (cross-field interaction) | [x] |
| 26 | `update_frame_header` | Boundary values on every axis simultaneously: BS/SR/CH/BD each drawn from `{0, 1, case-1, case, case+1, u32::MAX-1, u32::MAX}` sets, `channel_mode ∈ {0,1,2,3,4,255}` | [x] |
| 27 | `update_frame_header` | Batch/array shape: 1, 2, and 1024 consecutive `tflac` structs in one contiguous allocation, processed element-by-element by both libs, whole buffer compared (catches stride/size ABI mismatch) | [x] |
