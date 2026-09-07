# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/include/lib.h` (the whole public API) and every
branch / index / literal in `c_src/src/lib.c`.

## Axes the C actually branches on

**Runtime options / modes / flags:** *none.* The public header declares exactly
one function and no option struct, no init/config call, no global state, no
`enum`, and `src/lib.c` contains no `#if`/`#ifdef`. The only "option" the caller
can set is the `int nch` parameter, which selects the *stride* of the second
store (`pcm[16 * nch]`, `src/lib.c:33`). It is enumerated as its own axis.

**Public entry points (full set, lowest level included):**

| entry point | linkage | reachable from a `.so` consumer? |
|---|---|---|
| `synth_pair(mp3d_sample_t *pcm, int nch, const float *z)` | exported (`T`) | yes — the only one |
| `mp3d_scale_pcm(float)` | `static` (`src/lib.c:3`) | no; driven indirectly, **twice per call**, once per accumulator |

There is no convenience-wrapper / one-shot layer above `synth_pair` and no
lower-level exported primitive beneath it, so "exercise the low-level entry
points, not only the wrappers" is satisfied by driving `synth_pair` directly
across the axes below, which is also the only way to reach `mp3d_scale_pcm`.

**`nch` axis** (`int`, full `INT_MIN..=INT_MAX` domain is legal C input):
`1`, `2`, `3`, `0` (stores alias), negative (`-1`, `-2`), `INT_MAX` / `INT_MIN`
and other magnitudes where `16 * nch` **wraps in 32-bit `int`** (the C emits
`shl $0x4,%eax; cltq`, i.e. wrap-then-sign-extend).

**Input-shape axes for `z`** (the code special-cases values only through
`mp3d_scale_pcm`'s clamp, but the *accumulation* is value-dependent):

- which taps are non-zero: the first accumulator reads 15 taps
  `z[0], z[64], …, z[14*64]`; the second reads 8 taps at `z[2 + k*64]`,
  `k ∈ {0,2,4,6,8,10,12,14}` (after `z += 2`). Union = indices `0..=898`.
- magnitude class: exact zeros / subnormal / small / mid / large enough to
  saturate either clamp / exactly on a clamp boundary.
- sign pattern: the first accumulator mixes **subtractions**
  (`z[14*64]-z[0]`, `z[12*64]-z[2*64]`, `z[10*64]-z[4*64]`, `z[8*64]-z[6*64]`)
  with **additions**, so cancelling vs reinforcing sign patterns take different
  numeric paths; the second accumulator has **negative coefficients**
  (`-9975`, `-45`, `-5`).
- non-finite: `NaN`, `+Inf`, `-Inf`, and `+Inf`/`-Inf` combined so a
  subtraction produces `NaN`.
- buffer extent: exactly the minimum in-bounds length `899` floats vs a large
  padded buffer.
- independence of the two stores: configurations where store 0 clamps but store
  `16*nch` does not, and vice-versa.

## Rows

Every row is run through **both** `.so`s via `libloading` and compared
byte-for-byte on the whole `pcm` buffer. Rows marked *randomized* use
`N` pseudo-random inputs from a fixed-seed SplitMix64 (see
`tests/differential.rs`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `synth_pair` | `nch=1`; all 899 taps `0.0f` (degenerate exact-zero shape) | [x] |
| 2 | `synth_pair` | `nch=1`; *randomized* taps uniform in `[-1, 1]` — no clamp, small accumulators (4096 cases) | [x] |
| 3 | `synth_pair` | `nch=1`; *randomized* taps uniform in `[-4, 4]` — mid range, both stores near but mostly inside the clamp (4096 cases) | [x] |
| 4 | `synth_pair` | `nch=1`; *randomized* taps uniform in `[-64, 64]` — accumulators routinely exceed `±32767`, so **both clamp branches** fire at random (4096 cases) | [x] |
| 5 | `synth_pair` | `nch=1`; *randomized* full-range `f32` bit patterns restricted to finite values (`±1e-30 … ±1e30`) — extreme exponents, mixed magnitudes (4096 cases) | [x] |
| 6 | `synth_pair` | `nch=1`; *randomized* arbitrary 32-bit patterns reinterpreted as `f32`, **including NaN / Inf / subnormals** (4096 cases) | [x] |
| 7 | `synth_pair` | `nch=1`; sweep: exactly **one** non-zero tap at a time over all 899 indices × several magnitudes (isolates every coefficient, incl. the taps neither accumulator reads) | [x] |
| 8 | `synth_pair` | `nch=1`; taps chosen so accumulator 1 is driven to each **clamp boundary** exactly (`32766.5`, `-32767.5`) and one `f32` step either side | [x] |
| 9 | `synth_pair` | `nch=1`; taps chosen so accumulator 1 lands in the tricky `(-1, 0)` and `[-1, -0.5]` truncation windows and on `-0.0f` (the `s -= (s<0)` quirk) | [x] |
| 10 | `synth_pair` | `nch=1`; store 0 saturates **high** while store `16*nch` stays in range (independent-clamp interaction) | [x] |
| 11 | `synth_pair` | `nch=1`; store 0 saturates **low** while store `16*nch` saturates **high** (opposite clamps in one call) | [x] |
| 12 | `synth_pair` | `nch=2` (stereo stride, `pcm[32]`); *randomized* mid + saturating taps (2048 cases each) | [x] |
| 13 | `synth_pair` | `nch=3` (non-power-of-two stride, `pcm[48]`); *randomized* taps | [x] |
| 14 | `synth_pair` | `nch=0` — the two stores **alias** `pcm[0]`; verifies the *second* write wins and the store order matches | [x] |
| 15 | `synth_pair` | `nch=-1` and `nch=-2` — negative index, writes *before* `pcm` (must not wrap through `usize`); *randomized* taps | [x] |
| 16 | `synth_pair` | `nch` values whose `16*nch` **overflows 32-bit `int`**: `INT_MAX`, `INT_MIN`, `0x08000000`, `0x10000000`, `0x7FFFFFF0`, `-0x08000001` — the wrap-then-sign-extend offset | [x] |
| 17 | `synth_pair` | `nch=1`; `z` buffer of exactly the minimum in-bounds length (899 floats) at the end of a page-guarded region — proves neither side reads past `z[898]` | [x] |
| 18 | `synth_pair` | `nch=1`; `z` shape with **only the subtracted taps** non-zero (`z[0], z[2*64], z[4*64], z[6*64]`) and only the added taps non-zero, separately — pins the `+`/`-` pairing | [x] |
| 19 | `synth_pair` | `nch=1`; second accumulator's **negative-coefficient** taps only (`z[2+6*64]`, `z[2+4*64]`, `z[2+0*64]` → `-9975`, `-45`, `-5`), *randomized* | [x] |
| 20 | `synth_pair` | `nch=2`; *randomized* arbitrary bit-pattern taps (NaN/Inf/subnormal) × negative and overflowing `nch` — full cross of the value axis with the stride axis | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no `add_executable`**, and `translation/Cargo.toml` has no `[[bin]]` and no
`src/main.rs`. The project builds **no driver binary**, so the "compare C and
Rust stdout" item of the completion gate is not applicable (verified by
`grep -c add_executable c_src/CMakeLists.txt` → `0`).
