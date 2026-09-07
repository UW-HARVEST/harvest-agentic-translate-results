# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the `if` / `switch` branches in `c_src/src/lib.c`.

## Entry points (full set)

`nm -D` shows exactly one exported symbol, and it is also the only declaration
in the public header, so the "lowest-level entry point" and the "convenience
wrapper" are the same function. There are no wrappers to bypass.

| entry point | signature | source |
|-------------|-----------|--------|
| `update_frame_header` | `void update_frame_header(tflac *t)` | `include/lib.h:16`, `src/lib.c:11` |

## Axes the C actually branches on

The function has **no runtime option/mode/flag setters** and **no `#ifdef`**.
All configuration is carried in the `tflac` struct fields the function reads.
There are 4 independent branch axes plus 1 pure-input axis:

* **Axis BS — `cur_blocksize`** (`switch`, `src/lib.c:13`): 13 explicit cases
  (`192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768`)
  + a `default` that itself splits on `cur_blocksize <= 256` → **15 states**.
* **Axis SR — `samplerate`** (`switch`, `src/lib.c:58`): 11 explicit cases
  (`882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000`)
  + a `default` that splits 5 ways (`%1000==0 & /1000<256`, `%1000==0 & /1000>=256`
  (no-op), `<65536`, `%10==0 & /10<65536`, `%10==0 & /10>=65536` (no-op),
  `%10!=0` (no-op)) → **17 states**.
* **Axis CM — `channel_mode % 4`** (`switch`, `src/lib.c:107`): 4 reachable
  states (`0,1,2,3`); the `default` arm is unreachable because of the `% 4`.
  Only state `0` reads `channels`. → **4 states**.
* **Axis CH — `channels`** (only observable when `CM == 0`): `0` (wraps),
  `1`, `2..8` (legal FLAC), `9..15`, `16`+ (bits escape the nibble),
  `0x0FFFFFFF`+ (bits shift out of the u32) → **6 shapes**.
* **Axis BD — `bitdepth`** (`switch`, `src/lib.c:123`): 6 explicit cases
  (`8, 12, 16, 20, 24, 32`) + empty `default` → **7 states**.
* **Axis PRE — pre-call `frame_header` content**: `0`, `0xFFFFFFFF`, random.
  The first statement is `=` so this must never matter → **3 shapes**, and the
  whole 24-byte struct image (incl. the 3 padding bytes at offset 13..15) is
  compared after the call.

Full cross-product is 15·17·4·6·7·3 ≈ 385k; the rows below are the pruned set of
combinations the code actually distinguishes, and every row is driven with many
randomized inputs (fixed seed, deterministic xorshift PRNG) rather than one
hand-picked value.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `update_frame_header` | Axis BS sweep: each of the 13 explicit `cur_blocksize` cases, crossed with randomized `samplerate`/`channels`/`bitdepth`/`channel_mode` | [x] |
| 2 | `update_frame_header` | Axis BS `default`, `cur_blocksize <= 256`: randomized values in `0..=256` excluding the explicit cases (incl. `0`, `1`, `255`, `256` boundary) | [x] |
| 3 | `update_frame_header` | Axis BS `default`, `cur_blocksize > 256`: randomized values in `257..=u32::MAX` excluding explicit cases (incl. `257`, `u32::MAX`) | [x] |
| 4 | `update_frame_header` | Axis SR sweep: each of the 11 explicit `samplerate` cases, crossed with randomized other fields | [x] |
| 5 | `update_frame_header` | Axis SR `default` sub-branch `%1000==0 && /1000<256`: randomized `k*1000`, `k in 0..256`, `k*1000` not an explicit case (incl. `0`, `255000`) | [x] |
| 6 | `update_frame_header` | Axis SR `default` sub-branch `%1000==0 && /1000>=256` (silent no-op): randomized `k*1000`, `k >= 256` (incl. `256000`) | [x] |
| 7 | `update_frame_header` | Axis SR `default` sub-branch `%1000!=0 && <65536`: randomized `1..65535` with `%1000!=0`, not an explicit case (incl. `1`, `65535`) | [x] |
| 8 | `update_frame_header` | Axis SR `default` sub-branch `%1000!=0 && >=65536 && %10==0 && /10<65536`: randomized `k*10` in `65536..655360` with `%1000!=0` | [x] |
| 9 | `update_frame_header` | Axis SR `default` sub-branch `%1000!=0 && >=65536 && %10==0 && /10>=65536` (silent no-op): randomized `k*10 >= 655360` with `%1000!=0` | [x] |
| 10 | `update_frame_header` | Axis SR `default` sub-branch `%1000!=0 && >=65536 && %10!=0` (silent no-op): randomized `>=65536` with `%10!=0` (incl. `65537`, `u32::MAX`) | [x] |
| 11 | `update_frame_header` | Axis CM `= 0` (INDEPENDENT) × Axis CH `= 0`: `(channels-1)` u32 wrap to `0xFFFFFFF0` after shift | [x] |
| 12 | `update_frame_header` | Axis CM `= 0` × Axis CH `= 1..=8` (legal FLAC channel counts), all 8 values, randomized other fields | [x] |
| 13 | `update_frame_header` | Axis CM `= 0` × Axis CH `= 9..=16` (nibble overflow into bits 8+) | [x] |
| 14 | `update_frame_header` | Axis CM `= 0` × Axis CH randomized full `u32` range (incl. `>= 0x10000000` where `<< 4` truncates out of the u32) | [x] |
| 15 | `update_frame_header` | Axis CM `= 1` (LEFT_SIDE) with randomized `channels` — `channels` must be ignored | [x] |
| 16 | `update_frame_header` | Axis CM `= 2` (SIDE_RIGHT) with randomized `channels` — ignored | [x] |
| 17 | `update_frame_header` | Axis CM `= 3` (MID_SIDE) with randomized `channels` — ignored | [x] |
| 18 | `update_frame_header` | Axis CM: `channel_mode` swept over the FULL `u8` range `0..=255` (so every `%4` residue via every representative, incl. `4 == TFLAC_CHANNEL_MODE_COUNT` and `255`) × randomized other fields | [x] |
| 19 | `update_frame_header` | Axis BD sweep: each of the 6 explicit `bitdepth` cases × randomized other fields | [x] |
| 20 | `update_frame_header` | Axis BD `default` (empty): randomized `bitdepth` outside `{8,12,16,20,24,32}` (incl. `0`, `1`, `7`, `9`, `33`, `u32::MAX`) | [x] |
| 21 | `update_frame_header` | Axis PRE: `frame_header` pre-set to `0`, `0xFFFFFFFF`, and randomized garbage; must be fully overwritten | [x] |
| 22 | `update_frame_header` | Axis PRE: full 24-byte struct pre-filled with randomized bytes (incl. the 3 padding bytes at offsets 13..15); whole 24-byte image compared after the call | [x] |
| 23 | `update_frame_header` | Cross-product of BS boundary values × SR boundary values (all `{192,256,257,576,4608,32768,32769,0,1,u32::MAX}` × `{0,8000,22050,65535,65536,65537,256000,655350,655360,u32::MAX}`), other fields randomized | [x] |
| 24 | `update_frame_header` | Cross-product of all 4 `CM` residues × all 7 `BD` states × BS default-low/default-high, `channels` and `samplerate` randomized | [x] |
| 25 | `update_frame_header` | Unstructured fuzz: all 6 fields drawn from the full range of their type, 200 000 iterations, fixed seed — the whole 24-byte struct image compared | [x] |
| 26 | `update_frame_header` | Idempotence / repeated invocation: the same struct passed to the same `.so` twice, and C-then-Rust vs Rust-then-C orderings, on randomized inputs | [x] |
| 27 | `update_frame_header` | Aliasing/adjacency: struct embedded in a larger 72-byte buffer with guard bytes before and after; guard regions must be untouched by both | [x] |

All 27 rows pass across randomized inputs — see `tests/phase_b_configs.rs`
(25 `#[test]` functions: rows 15, 16 and 17 share
`cfg_15_16_17_stereo_modes_ignore_channels`, which loops over the three
non-zero `%4` residues).

## Exhaustive confirmation (`tests/phase_b_exhaustive.rs`)

Because each axis contributes to a **disjoint bit range** of `frame_header` via
`|=`, a per-axis exhaustive sweep is a strong global check — and it is cheap
enough to actually run. Each of the four `u32` axes was swept over its
**complete `2^32` domain**, comparing the full 24-byte struct image from both
`.so`s on every value (~17.2 billion comparisons, 69 s wall clock):

```
cur_blocksize: exhaustive 2^32 sweep clean
channels:      exhaustive 2^32 sweep clean
bitdepth:      exhaustive 2^32 sweep clean
samplerate:    exhaustive 2^32 sweep clean
test result: ok. 4 passed; 0 failed; finished in 69.32s
```

`channel_mode` is a `u8` and is swept exhaustively (all 256 values) crossed with
every `channels` boundary in `exhaustive_channel_mode_x_channels`, which runs by
default. The `u32` sweeps are `#[ignore]`d for runtime; run them with:

```
cargo test --release --offline --test phase_b_exhaustive -- --ignored --nocapture
```

## Running everything

`./verify.sh` builds the C `.so`, enumerates the feature combinations from
`Cargo.toml`, builds the Rust cdylib in **both** the release and the debug
profile, checks the `nm -D` symbol diff, and runs the whole suite for every
combination. The debug profile matters: Rust's arithmetic-overflow checks are
enabled there, and the C relies on silent `uint32_t` wrapping in
`(t->channels - 1)`, so a plain `-` instead of `wrapping_sub` would panic in
debug while passing in release. Both profiles pass.
