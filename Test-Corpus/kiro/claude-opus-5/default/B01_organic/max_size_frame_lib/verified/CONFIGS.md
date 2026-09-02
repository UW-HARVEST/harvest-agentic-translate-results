# CONFIGS.md — configuration-surface table

Derived mechanically from the single C expression in `c_src/src/lib.c`:

```c
tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth) {
    return 18U + channels +
           (((blocksize * bitdepth * (channels * (channels != 2))) +   /* term1 */
             (blocksize * bitdepth * (channels == 2)) +                /* term2 */
             (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2)) + /* term3 */
             +7) /
            8);
}
```

## Axes the C actually branches on

There is no runtime option struct, no mode/flag setter, no `#ifdef`, and no
`if`/`switch` — the header exposes one entry point and it is also the lowest-level
entry point, so "convenience wrapper vs low-level API" does not apply. The
branching is entirely *value*-driven, through three relational sub-expressions
used as 0/1 multipliers:

| axis | source of the branch | distinct states |
|---|---|---|
| A. stereo flag | `(channels == 2)` / `(channels != 2)` | `channels == 2`, `channels != 2` |
| B. 32-bit-depth flag | `(bitdepth != 32)` | `bitdepth == 32`, `bitdepth != 32`; **only observable when `channels == 2`**, since term3 is multiplied by `(channels == 2)` |
| C. `channels` shape | `channels` appears as a bare factor in term1 and as an addend in `18U + channels` | `0`, `1`, `2`, `3`, many (`>3`), huge (wraps a multiply) |
| D. `bitdepth` shape (width) | bare factor in term1/term2, `bitdepth + flag` in term3 | `0`, `1`, typical widths `{8,16,24}`, boundary `{31,32,33}`, huge |
| E. `blocksize` shape (count) | bare factor in all three terms | `0` (empty), `1` (one), typical `{16,4096}`, `65535`, huge |
| F. arithmetic regime | all ops are `uint32_t`, so each `*`/`+` can wrap mod 2^32 | no-overflow, term wraps, `+7` wraps, final `18+channels+bytes` wraps |
| G. division truncation | `/ 8` on the numerator | numerator `< 8`, numerator `% 8 == 0`, numerator `% 8 != 0` |

Axis B is *degenerate* for `channels != 2`: term3 vanishes, so `bitdepth == 32`
and `bitdepth == 33` differ only through term1/term2. Rows below keep both
sub-cases for the stereo branch (where the flag is live) and prune the
non-stereo duplicates down to representative widths.

## Rows (pruned cross-product)

Each row is exercised with **many randomized inputs** on the free axes plus the
fixed pinned values, seeded deterministically (`SEED = 0x5F3D_C0DE_1234_5678`,
custom SplitMix64 so the results are reproducible without external crates).
Both `.so`s are loaded with `libloading` and the `u32` results compared exactly.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `max_size_frame` | `channels == 2` (stereo, term2+term3 live), `bitdepth == 32` (so `bitdepth != 32` flag = 0), `blocksize` randomized over typical range `1..=65535` | `cfg_c1` | [x] |
| C2 | `max_size_frame` | `channels == 2`, `bitdepth != 32` randomized over `1..=31` (flag = 1, term3 uses `bitdepth+1`), `blocksize` randomized `1..=65535` | `cfg_c2` | [x] |
| C3 | `max_size_frame` | `channels == 2`, `bitdepth` randomized `33..=64` (above the boundary, flag = 1), `blocksize` randomized `1..=65535` | `cfg_c3` | [x] |
| C4 | `max_size_frame` | `channels == 2`, `bitdepth == 0` (zero width, flag still 1 so term3 = `blocksize*1`), `blocksize` randomized full `u32` | `cfg_c4` | [x] |
| C5 | `max_size_frame` | `channels == 2`, `blocksize == 0` (empty), `bitdepth` randomized full `u32` | `cfg_c5` | [x] |
| C6 | `max_size_frame` | `channels == 2`, `blocksize == 1` (single sample), `bitdepth` randomized full `u32` incl. `32` | `cfg_c6` | [x] |
| C7 | `max_size_frame` | `channels == 0` (no channels — term1 zeroed by the bare `channels` factor, term2/term3 zeroed by the stereo flag), `blocksize`/`bitdepth` randomized full `u32` | `cfg_c7` | [x] |
| C8 | `max_size_frame` | `channels == 1` (mono, term1 live with factor 1), `bitdepth` randomized over `{1..=64}` incl. `31/32/33`, `blocksize` randomized `1..=65535` | `cfg_c8` | [x] |
| C9 | `max_size_frame` | `channels == 3` (one past the stereo special case, term1 live with factor 3), `bitdepth` randomized `1..=64`, `blocksize` randomized `1..=65535` | `cfg_c9` | [x] |
| C10 | `max_size_frame` | `channels` randomized `4..=255` (many channels, non-stereo), `bitdepth` randomized `1..=64`, `blocksize` randomized `1..=65535` | `cfg_c10` | [x] |
| C11 | `max_size_frame` | non-stereo with `bitdepth == 32` exactly — confirms the `bitdepth != 32` flag is unobservable off the stereo path; `channels` randomized from `{0,1,3,4,..}`, `blocksize` randomized | `cfg_c11` | [x] |
| C12 | `max_size_frame` | typical FLAC-like shapes: `blocksize ∈ {192, 576, 1152, 2304, 4096, 4608}`, `channels ∈ 1..=8`, `bitdepth ∈ {8,12,16,20,24,32}` — full cross-product, exhaustive not random | `cfg_c12` | [x] |
| C13 | `max_size_frame` | boundary triple sweep: `channels ∈ {0,1,2,3}` × `bitdepth ∈ {0,1,31,32,33}` × `blocksize ∈ {0,1,2,7,8,9,65535,65536}` — exhaustive over all 160 combinations | `cfg_c13` | [x] |
| C14 | `max_size_frame` | division-truncation regime: `blocksize`/`bitdepth`/`channels` chosen so the numerator sweeps every residue class mod 8 (`numerator % 8 ∈ 0..=7`), both stereo and mono | `cfg_c14` | [x] |
| C15 | `max_size_frame` | overflow regime, stereo: `blocksize` and `bitdepth` randomized in `2^16..=2^32-1` so `blocksize*bitdepth` wraps mod 2^32 | `cfg_c15` | [x] |
| C16 | `max_size_frame` | overflow regime, non-stereo: `channels` randomized in `2^16..=2^32-1` (huge channel count) so `blocksize*bitdepth*channels` wraps, and `18+channels` also wraps | `cfg_c16` | [x] |
| C17 | `max_size_frame` | `bitdepth == u32::MAX` — `bitdepth + (bitdepth != 32)` itself wraps to `0` inside term3; stereo so term3 is live | `cfg_c17` | [x] |
| C18 | `max_size_frame` | unconstrained fuzz: all three arguments uniformly random over the full `u32` range (hits mixed wrap/no-wrap regimes), large iteration count | `cfg_c18` | [x] |
| C19 | `max_size_frame` | "interesting constants" cross-product: each argument drawn from `{0,1,2,3,7,8,31,32,33,255,256,65535,65536,0x7FFF_FFFF,0x8000_0000,0xFFFF_FFFE,0xFFFF_FFFF}` — exhaustive 16^3 = 4096 combinations | `cfg_c19` | [x] |
| C20 | `max_size_frame` | repeat-call / statelessness check: the same configuration invoked repeatedly and interleaved with other configurations, asserting both libraries stay in lockstep (no hidden state in either `.so`) | `cfg_c20` | [x] |

All 20 rows have a passing differential test in `tests/differential.rs`; see the
`valid_paths` module.
