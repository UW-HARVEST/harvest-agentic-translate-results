# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

There is exactly **one** public entry point (`c_src/include/lib.h`), and it is
also the lowest-level one — the two internal helpers `cbLuminance` and
`cbContrastRatio` are `static`, so `contrast_ratio` is simultaneously the
convenience wrapper *and* the deepest reachable entry point. There is nothing
lower to drive directly.

There are **no runtime options, modes, or flags**: no setter, no context
struct, no global state, no `#ifdef`, no `switch` (all greps = 0, see
`ERRORS.md`). The entire branch surface of the library is:

* **Axis 1 — per-channel transfer branch** (3 ternaries in `cbLuminance`):
  `C > 0.04045 ? pow((C+0.055)/1.055, 2.4) : C/12.92`, where `C = byte/255.f`.
  Threshold: `0.04045 * 255 = 10.31475`, so byte `<= 10` → **linear** branch,
  byte `>= 11` → **pow** branch. Applied independently to R, G, B → **2^3 = 8
  branch patterns per color**, and there are two colors → 64 pattern pairs.
* **Axis 2 — swap branch** (the single `if` in `cbContrastRatio`):
  `LumA < LumB` (swap) / `LumA > LumB` (no swap) / `LumA == LumB` (no swap).
* **Axis 3 — divisor state** at the unguarded `High / Low`:
  `Low > 0` / `Low == 0` (black) / both `0`.
* **Axis 4 — channel value shape** within a branch: boundary bytes
  `0, 1, 10, 11, 254, 255` vs interior values; per-channel isolation (only one
  channel nonzero) exposes the individual `0.2126f / 0.7152f / 0.0722f`
  weights and the left-to-right single-precision accumulation order.
* **Axis 5 — ABI shape**: the 3-byte `cb_rgb_255` is passed by value in one
  x86-64 SysV INTEGER register; the 5 unused high bytes are a real input the
  caller controls.

Rows below are the cross-product of these axes, pruned to the combinations the
C actually distinguishes. Every row is driven with **many randomized inputs**
from a fixed-seed PRNG (a SplitMix64, seed `0x243F6A8885A308D3`) unless the row
is by definition a single point, and every row asserts **byte-for-byte**
(`to_bits()`) equality of the C and Rust return values, NaN included.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|--------------------------------------------|-----|
| 1 | `contrast_ratio` | **Exhaustive** grayscale sweep: all 256 bytes `v` as `A=(v,v,v)` against fixed white `B=(255,255,255)` — crosses the byte-10/11 threshold on all three channels simultaneously | [x] |
| 2 | `contrast_ratio` | **Exhaustive** grayscale sweep in the swapped orientation: fixed white `A` against all 256 `B=(v,v,v)` | [x] |
| 3 | `contrast_ratio` | **Exhaustive** per-channel isolation, R only: all 256 `A=(v,0,0)` vs white — isolates the `0.2126f` weight and the linear/pow branch on R while G,B stay on the linear branch | [x] |
| 4 | `contrast_ratio` | **Exhaustive** per-channel isolation, G only: all 256 `A=(0,v,0)` vs white — isolates `0.7152f` | [x] |
| 5 | `contrast_ratio` | **Exhaustive** per-channel isolation, B only: all 256 `A=(0,0,v)` vs white — isolates `0.0722f` | [x] |
| 6 | `contrast_ratio` | **Exhaustive** 2-channel sweep `A=(i,j,0)` over all 65 536 `(i,j)` vs white — mixed linear/pow across two channels at once | [x] |
| 7 | `contrast_ratio` | **Exhaustive over the whole luminance domain**: all 2^24 colors `A=(r,g,b)` vs fixed mid-gray `B=(128,128,128)`. Since the result depends on `A` only through `LumA`, this exhausts every reachable luminance value on the A side | [x] |
| 8 | `contrast_ratio` | **Exhaustive over the whole luminance domain, B side**: all 2^24 colors `B=(r,g,b)` vs fixed mid-gray `A`, exercising the swap branch across the full domain | [x] |
| 9 | `contrast_ratio` | All 64 transfer-branch pattern pairs: for each of the 8 linear/pow patterns for `A` × 8 for `B`, randomized bytes drawn *within* the branch each channel's pattern selects (`0..=10` for linear, `11..=255` for pow) | [x] |
| 10 | `contrast_ratio` | Axis 2 = swap taken (`LumA < LumB`), randomized pairs filtered to that relation, `Low > 0` | [x] |
| 11 | `contrast_ratio` | Axis 2 = no swap (`LumA > LumB`), randomized pairs filtered to that relation, `Low > 0` | [x] |
| 12 | `contrast_ratio` | Axis 2 = `LumA == LumB` with `A != B` (distinct colors of equal luminance, found by search) → result exactly `1.0f`, no swap | [x] |
| 13 | `contrast_ratio` | Axis 2 = `LumA == LumB` with `A == B`, randomized non-black colors → exactly `1.0f` | [x] |
| 14 | `contrast_ratio` | Axis 3 = `Low == 0`, black on the `B` side: randomized non-black `A` vs `B=(0,0,0)` → `+inf` | [x] |
| 15 | `contrast_ratio` | Axis 3 = `Low == 0`, black on the `A` side: `A=(0,0,0)` vs randomized non-black `B` → swap then `+inf` | [x] |
| 16 | `contrast_ratio` | Axis 3 = both zero: `A=B=(0,0,0)` → `NaN` | [x] |
| 17 | `contrast_ratio` | Boundary bytes only: both colors drawn from the boundary set `{0,1,10,11,254,255}` on every channel — the full 6^3 × 6^3 corner grid, randomized sample plus all-corners enumeration | [x] |
| 18 | `contrast_ratio` | Threshold pair straddle: for each channel position, `A` byte `10` (linear) vs `B` byte `11` (pow), all three positions | [x] |
| 19 | `contrast_ratio` | Fully randomized unconstrained pairs (both colors uniform over all 2^24), large sample — the default consumer usage | [x] |
| 20 | `contrast_ratio` | Randomized pairs restricted to the **dark** subdomain (all channels `0..=10`, i.e. all six channels on the linear branch, small denominators) | [x] |
| 21 | `contrast_ratio` | Randomized pairs restricted to the **bright** subdomain (all channels `245..=255`, all six channels on the pow branch, ratios near `1.0`) | [x] |
| 22 | `contrast_ratio` | Extreme-contrast corners: the 8 pure-primary/secondary colors (`black, red, green, blue, cyan, magenta, yellow, white`) crossed with themselves — all 64 ordered pairs, covering `inf`, `NaN`, `1.0`, and asymmetric pairs | [x] |
| 23 | `contrast_ratio` | Symmetry property under both implementations: `f(A,B)` and `f(B,A)` compared C-vs-Rust in both orientations over randomized pairs (the `if` makes the C symmetric; Rust must be symmetric in the *same* way) | [x] |
| 24 | `contrast_ratio` | Axis 5: clean call vs call with garbage in the unused high bytes of argument register 1, randomized colors, via an `extern "C" fn(u64, u64) -> f32` view of the same symbol | [x] |
| 25 | `contrast_ratio` | Axis 5: garbage in the unused high bytes of argument register 2 | [x] |
| 26 | `contrast_ratio` | Axis 5: garbage in the unused high bytes of **both** argument registers, randomized colors and randomized garbage | [x] |
| 27 | `contrast_ratio` | Repeated-call statelessness: the same input called interleaved with many other inputs returns identical bits every time on both sides (no hidden global/`static` state — confirmed absent by grep, asserted anyway) | [x] |
