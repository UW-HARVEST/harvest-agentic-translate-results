# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Mechanical derivation of the axes

The public API is one function (see `SYMBOLS.md`); it is simultaneously the
convenience wrapper *and* the lowest-level entry point — there is no deeper
layer to reach past, and no state/handle/option struct to configure
(`grep` for `struct`/`enum`/`#define`/`if`/`switch` in `c_src` → no matches).
So the configuration surface is entirely the **shape of the three arguments**,
and the axes are exactly the sub-expressions the C branches or annihilates on:

| axis | source line | distinct classes the C treats differently |
|---|---|---|
| **A. `channels` vs 2** | `channels * (channels != 2)`, `(channels == 2)` ×2 | `== 2` (stereo: `t1` zeroed, `t2`+`t3` active) vs `!= 2` (`t2`,`t3` zeroed, `t1` active) |
| **B. `channels == 0`** | `channels * (...)` | `0` annihilates `t1` even on the `!= 2` branch → constant `18` |
| **C. `bitdepth` vs 32** | `bitdepth + (bitdepth != 32)` | `== 32` (no `+1`) vs `!= 32` (`+1`); only observable when `channels == 2` |
| **D. `bitdepth == 0` / `blocksize == 0`** | the products | annihilate terms |
| **E. wraparound regime** | all `*` and `+` are `uint32_t` | products fit in 32 bits vs wrap mod 2³² (incl. `bitdepth+1` wrapping `MAX`→`0`, and `18+channels` wrapping) |
| **F. `(… + 7) / 8`** | division line | the 8 residues of `sum mod 8` (truncating divide, i.e. ceiling of `sum/8` pre-`+7`) |
| **G. magnitude classes** | argument domain is full `uint32_t` | `0`, `1`, `2`, small, typical-FLAC (`bs∈{16,4096,65535}`, `ch∈{1..8}`, `bd∈{4,8,12,16,20,24,32}`), one-past-range (`ch=9`, `bd=33`, `bs=65536`), huge (`2³¹`), `MAX` |

Rows below are the pruned cross-product A×B×C×D×E×F×G — one row per
combination the C actually distinguishes. Every row is driven with **many
randomized inputs** (SplitMix64, fixed seed `0x5F3759DF_C0FFEE01`) over the
free axes of that row, not a single hand-picked value, and compared C-vs-Rust
byte-for-byte through the `.so` exports.

## Configuration surface table

Argument order `(blocksize, channels, bitdepth)`; `MAX` = `0xFFFFFFFF`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `max_size_frame` | stereo (`ch=2`) × `bd=32` (no `+1`) × `bs` random in `1..=65535` | [x] |
| C2 | `max_size_frame` | stereo (`ch=2`) × `bd` random in typical `{4,8,12,16,20,24}` (`+1` applies) × `bs` random `1..=65535` | [x] |
| C3 | `max_size_frame` | stereo (`ch=2`) × `bd=0` × `bs` random `0..=65535` (D: `t2` dies, `t3=bs`) | [x] |
| C4 | `max_size_frame` | stereo (`ch=2`) × `bd=31` and `bd=33` (both sides of the 32 boundary) × random `bs` | [x] |
| C5 | `max_size_frame` | stereo (`ch=2`) × `bd=MAX` (E: `bd+1` wraps to 0, `t3` dies) × random `bs` | [x] |
| C6 | `max_size_frame` | stereo (`ch=2`) × `bs=0` × `bd` random full-`u32` | [x] |
| C7 | `max_size_frame` | stereo (`ch=2`) × `bs`,`bd` random full-`u32` (wrap regime dominant) | [x] |
| C8 | `max_size_frame` | mono (`ch=1`) × `bd=32` × `bs` random `1..=65535` | [x] |
| C9 | `max_size_frame` | mono (`ch=1`) × `bd` random typical × `bs` random `1..=65535` (note: `bd!=32` `+1` must **not** apply) | [x] |
| C10 | `max_size_frame` | mono (`ch=1`) × `bd=MAX` × random `bs` (proves `bd+1` wrap is unused off the stereo branch) | [x] |
| C11 | `max_size_frame` | `ch=0` × `bs`,`bd` random full-`u32` (B: must always be `18`) | [x] |
| C12 | `max_size_frame` | `ch=3` (just past stereo) × `bd` random typical × `bs` random `1..=65535` | [x] |
| C13 | `max_size_frame` | `ch` random in `4..=8` (multichannel FLAC) × `bd` random typical × `bs` random `1..=65535` | [x] |
| C14 | `max_size_frame` | `ch=9` (one past FLAC max) × `bd` random typical × random `bs` | [x] |
| C15 | `max_size_frame` | `ch` random `10..=255` × `bd` random typical × random `bs` | [x] |
| C16 | `max_size_frame` | `ch` random `256..=65535` × random `bd` × random `bs` (product wrap likely) | [x] |
| C17 | `max_size_frame` | `ch=MAX` (E: `18+ch` wraps) × `bs`,`bd` random full-`u32` | [x] |
| C18 | `max_size_frame` | `ch` random in `MAX-17 ..= MAX` (sweeps `18+ch` across the wrap point) × random `bs`,`bd` | [x] |
| C19 | `max_size_frame` | `bs=0` × `ch` random full-`u32` × `bd` random full-`u32` (D on blocksize) | [x] |
| C20 | `max_size_frame` | `bs=1` × `ch` random `0..=8` × `bd` random `0..=33` (smallest non-empty block, dense small grid) | [x] |
| C21 | `max_size_frame` | `bs=65535` (FLAC max) and `bs=65536` (one past) × `ch` random `1..=8` × `bd` random typical | [x] |
| C22 | `max_size_frame` | `bs=2^31` / `2^32-1` (huge) × random `ch` `1..=8` × random typical `bd` (E: heavy wrap) | [x] |
| C23 | `max_size_frame` | F: `bs` swept `0..=64` with `ch=1,bd=1` — every residue of `sum mod 8`, both branches | [x] |
| C24 | `max_size_frame` | F: `bs` swept `0..=64` with `ch=2,bd=1` — residues on the stereo branch (`t2+t3` both odd) | [x] |
| C25 | `max_size_frame` | unconstrained: all three args random full-`u32`, 2,000,000 draws (global wrap-regime fuzz) | [x] |
| C26 | `max_size_frame` | powers-of-two / bit-pattern grid: each arg from `{0,1,2,3,7,8,2^k, 2^k±1, MAX}` cross-product (exhaustive over the interesting-value set) | [x] |
| C27 | `max_size_frame` | exhaustive dense cube: `bs ∈ 0..=40`, `ch ∈ 0..=40`, `bd ∈ 0..=40` (68,921 combos, every small-value interaction) | [x] |
| C28 | `max_size_frame` | realistic FLAC matrix: full cross-product `bs ∈ {192,576,1152,2304,4608,4096,8192,16384,65535}` × `ch ∈ 1..=8` × `bd ∈ {4,8,12,16,20,24,32}` | [x] |
| C29 | `max_size_frame` | repeated-call / stateless check: same args called 1000× interleaved between C and Rust (proves no hidden state or init order dependence in either `.so`) | [x] |
| C30 | `max_size_frame` | ABI/calling-convention check: return value read as `u32` and arguments passed at register boundaries (values with high bit set in every position, e.g. `0x80000000`) to catch sign-extension mismatches | [x] |

All 30 rows are exercised by `tests/configs.rs` (Phase B).
