# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Axes are derived from what the C in
`c_src/src/lib.c` actually branches on / indexes with, not from guesses.

## Entry points (complete)

| entry point | linkage | reachable how |
|-------------|---------|---------------|
| `synth_pair(mp3d_sample_t *pcm, int nch, const float *z)` | exported (`nm -D`) | called directly via `dlsym` |
| `mp3d_scale_pcm(float)` | `static`, lowest-level helper | not exported; driven through **both** of its call sites (`pcm[0]` and `pcm[16*nch]`) by constructing `z` so that a chosen accumulator value reaches it |

There are no convenience wrappers to hide behind: `synth_pair` *is* the
low-level entry point, and the tests always drive it directly. `mp3d_scale_pcm`
is exercised through both call sites separately, because the two call sites feed
it from two different accumulation chains.

## Axes the C code distinguishes

* **A. `nch`** — used only as `pcm[16 * nch]`; the C computes the index in `int`.
  Distinct shapes: `0` (aliases `pcm[0]`), `1` (mono), `2` (stereo), other
  positive, negative, and `int`-overflowing.
* **B. Which output tap** — two independent accumulation chains with different
  coefficient sets and different `z` indexing (`z[i*64]` vs `(z+2)[i*64]`), so
  every data shape must be checked against *both*.
* **C. `z` tap index set** — 15 taps at `i*64` (`i = 0..14`) for chain 0, and
  only **8** taps at `2 + i*64` for chain 1, because that chain reads just the
  *even* multiples (`i = 0, 2, 4, 6, 8, 10, 12, 14`). That is 23 of the 899
  reachable floats; everything else must be untouched. (Confirmed against the
  real C by `harness_model_matches_c_library` in `tests/probe.rs`.)
* **D. Accumulator regime feeding `mp3d_scale_pcm`** — in-range positive,
  in-range negative, `|a|` clamped high, clamped low, exactly on a `.5`
  rounding tie, exactly on the `32766.5` / `-32767.5` clamp boundary, `±inf`,
  `NaN`.
* **E. `z` value class** — `+0.0`, `-0.0`, subnormal, tiny normal, unit-scale,
  large, `±FLT_MAX`, `±inf`, `NaN`, arbitrary bit patterns.
* **F. Accumulation order sensitivity** — 8 sequential `float` `+=` steps per
  output; catastrophic cancellation and intermediate overflow make the result
  order-dependent, so re-association in the translation must be detected.
* **G. `z` pointer alignment** — `z + 2` inside the function means the second
  chain reads at an 8-byte offset; the incoming `z` may itself be 4-byte (not
  16-byte) aligned.
* **H. Statelessness** — no globals in C, so repeated/interleaved calls must be
  independent.

Every row is driven with **many** randomized inputs (fixed seeds, so failures
reproduce), not one hand-picked vector, and `z`'s 876 *unread* slots are
poisoned with `NaN` so any index mistake in the translation collapses the Rust
accumulator to `0` and shows up as an immediate divergence.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `synth_pair` | `nch=1`; all 23 read taps `+0.0`, all unread slots `NaN` — proves the exact read-index set (C1, C) | [x] |
| 2  | `synth_pair` | `nch=1`; all read taps `-0.0` (E: signed zero) | [x] |
| 3  | `synth_pair` | `nch=2`; taps uniform in `±1.0`, 2000 random vectors (E unit-scale, D in-range) | [x] |
| 4  | `synth_pair` | `nch=2`; taps uniform in `±0.5`, 2000 vectors — mid-scale PCM, mixes in-range and clamped | [x] |
| 5  | `synth_pair` | `nch=1`; taps log-uniform `1e-8 … 1e-1`, 2000 vectors (E tiny; D near-zero rounding) | [x] |
| 6  | `synth_pair` | `nch=1`; taps log-uniform `1e-1 … 1e2`, 2000 vectors (D straddles both clamps) | [x] |
| 7  | `synth_pair` | `nch=2`; taps log-uniform `1e2 … 1e6`, 2000 vectors (D clamp-high/clamp-low dominant) | [x] |
| 8  | `synth_pair` | `nch=2`; taps = arbitrary random bit patterns incl. `NaN`/`inf`/subnormal, 4000 vectors (E full) | [x] |
| 9  | `synth_pair` | `nch=1`; taps = random subnormals (`f32::MIN_POSITIVE` scaled down), 500 vectors (E subnormal) | [x] |
| 10 | `synth_pair` | `nch=2`; taps = `±f32::MAX` random signs, 500 vectors — accumulator overflows to `±inf` mid-chain (F, D) | [x] |
| 11 | `synth_pair` (→ `mp3d_scale_pcm` call site 1) | single-tap activation: for each of the 8 coefficients of chain 0 (`29, 213, 459, 2037, 5153, 6574, 37489, 75038`), 300 random magnitudes each — isolates every index/weight pair (B, C) | [x] |
| 12 | `synth_pair` (→ `mp3d_scale_pcm` call site 2) | single-tap activation: for each of the 8 coefficients of chain 1 (`104, 1567, 9727, 64019, -9975, -45, 146, -5`) at `z+2` offsets, 300 random magnitudes each (B, C) | [x] |
| 13 | `synth_pair` | `nch=2`; each of the 23 read taps in turn driven to a huge value with the rest random — sign/subtraction structure per index (B, C, F) | [x] |
| 14 | `synth_pair` | `nch=1`; chain-0 accumulator swept over `±4.0` in fine steps via `z[7*64]` — every rounding tie and the `s -= (s<0)` branch (D tie, F) | [x] |
| 15 | `synth_pair` | `nch=1`; chain-0 accumulator swept ULP-by-ULP through the `32766.5` clamp boundary, both signs (D boundary) | [x] |
| 16 | `synth_pair` | `nch=1`; chain-1 accumulator swept ULP-by-ULP through the `32766.5` / `-32767.5` boundaries via `z[2+8*64]` (D boundary, B) | [x] |
| 17 | `synth_pair` | `nch=1`; both chains simultaneously driven to independent random clamp/in-range regimes, 2000 vectors (B × D cross-product) | [x] |
| 18 | `synth_pair` | `nch=0` — second store aliases `pcm[0]`; random taps, 500 vectors (A) | [x] |
| 19 | `synth_pair` | `nch ∈ {-1, -2, -8, -64}` with `pcm` based inside the buffer; random taps (A negative) | [x] |
| 20 | `synth_pair` | `nch ∈ {1, 2, 3, 4, 8, 16, 64, 1024}` — full positive spread; random taps (A) | [x] |
| 21 | `synth_pair` | `nch=2`; `z` deliberately unaligned (offset 1/2/3 floats into a larger allocation), random taps (G) | [x] |
| 22 | `synth_pair` | `nch=2`; taps engineered for catastrophic cancellation (huge equal-and-opposite partial sums) — detects any re-association (F) | [x] |
| 23 | `synth_pair` | `nch=2`; taps engineered so an intermediate sum overflows to `inf` and the next term is `-inf`-producing (order-dependent `NaN`) (F) | [x] |
| 24 | `synth_pair` | statelessness: 300 random configurations replayed in a different, interleaved order and re-checked (H) | [x] |
| 25 | `synth_pair` | `nch=2`; `pcm` prefilled with poison, only the two expected slots may change — verifies no extra stores (A, B) | [x] |
| 26 | `synth_pair` | mixed randomized fuzz across the whole axis cross-product (`nch` from A × value class from E), 20000 iterations, single fixed seed | [x] |
