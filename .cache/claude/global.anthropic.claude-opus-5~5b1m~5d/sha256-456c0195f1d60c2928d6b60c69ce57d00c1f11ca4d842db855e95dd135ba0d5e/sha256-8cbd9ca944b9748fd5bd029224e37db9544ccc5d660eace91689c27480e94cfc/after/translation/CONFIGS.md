# CONFIGS.md — configuration surface table (Phase B)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

The library exposes **no** runtime options, flags, modes, contexts, byte-order
selectors or `#ifdef`s (`grep -n '#if\|#ifdef\|enum\|extern' c_src/src/lib.c` →
only `#include`s). The Cargo manifest declares **no `[features]`**, so there is
exactly one feature combination (`--no-default-features` == default == all).
The configuration surface is therefore entirely made of *input shapes*:

* **Axis 1 — sRGB transfer branch, per channel (6 independent instances).**
  `X > 0.04045 ? pow((X+0.055)/1.055, 2.4) : X/12.92`, evaluated after
  promotion to `double`. For `X = byte/255f`: byte ≤ 10 ⇒ **L** (linear arm),
  byte ≥ 11 ⇒ **P** (`pow` arm). Each colour has a 3-bit mask over (R,G,B),
  giving 8 states per colour: `LLL, LLP, LPL, LPP, PLL, PLP, PPL, PPP`.
* **Axis 2 — the `if (High < Low)` swap in `cbContrastRatio`**: taken
  (LumA < LumB) / not taken (LumA >= LumB, or either operand NaN).
* **Axis 3 — entry point.** `contrast_ratio` is the *only* public entry point
  (the lower-level `cbLuminance` / `cbContrastRatio` are `static`, hence not
  callable across the FFI boundary; they are exercised transitively and their
  intermediate values are pinned by the exhaustive grey-scale sweep, row 73).
* **Axis 4 — value shape**: all-zero channels, all-max channels, equal args,
  single-channel-only colours, greys, exhaustive per-byte sweeps.

Rows 1–64 are the full cross product of Axis 1 (colour A mask × colour B mask);
Axis 2 is covered *inside every one of those rows* by testing both argument
orders `(A,B)` and `(B,A)` for every randomized sample. Rows 65+ add the
remaining shape axes.

Every row is driven with **many randomized inputs** (`SplitMix64`, fixed seed
`0x5EED_1234_ABCD_0001`, ≥256 samples/row) whose bytes are drawn from the
byte sub-range that realises the required branch mask, and both libraries are
compared on **raw `u32` bit patterns**.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 2 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 3 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 4 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 5 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 6 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 7 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 8 | `contrast_ratio` | A branch mask `LLL` (R=lin,G=lin,B=lin) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 9 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 10 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 11 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 12 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 13 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 14 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 15 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 16 | `contrast_ratio` | A branch mask `LLP` (R=lin,G=lin,B=pow) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 17 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 18 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 19 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 20 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 21 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 22 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 23 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 24 | `contrast_ratio` | A branch mask `LPL` (R=lin,G=pow,B=lin) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 25 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 26 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 27 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 28 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 29 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 30 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 31 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 32 | `contrast_ratio` | A branch mask `LPP` (R=lin,G=pow,B=pow) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 33 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 34 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 35 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 36 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 37 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 38 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 39 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 40 | `contrast_ratio` | A branch mask `PLL` (R=pow,G=lin,B=lin) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 41 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 42 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 43 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 44 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 45 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 46 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 47 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 48 | `contrast_ratio` | A branch mask `PLP` (R=pow,G=lin,B=pow) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 49 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 50 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 51 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 52 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 53 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 54 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 55 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 56 | `contrast_ratio` | A branch mask `PPL` (R=pow,G=pow,B=lin) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 57 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `LLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 58 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `LLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 59 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `LPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 60 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `LPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 61 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `PLL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 62 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `PLP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 63 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `PPL`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 64 | `contrast_ratio` | A branch mask `PPP` (R=pow,G=pow,B=pow) × B branch mask `PPP`; randomized bytes in the arm-selecting sub-ranges; both argument orders (swap taken and not taken) | [x] |
| 65 | `contrast_ratio` | exhaustive: every grey pair `A={v,v,v}`, `B={w,w,w}` for all 256×256 = 65536 `(v,w)` — pins `cbLuminance` and the swap for every byte value | [x] |
| 66 | `contrast_ratio` | exhaustive single-axis sweep: `A={v,0,0}` vs fixed `B={128,128,128}` for all 256 `v` (red channel only) | [x] |
| 67 | `contrast_ratio` | exhaustive single-axis sweep: `A={0,v,0}` vs fixed `B={128,128,128}` for all 256 `v` (green channel only) | [x] |
| 68 | `contrast_ratio` | exhaustive single-axis sweep: `A={0,0,v}` vs fixed `B={128,128,128}` for all 256 `v` (blue channel only) | [x] |
| 69 | `contrast_ratio` | exhaustive single-axis sweep on the *second* argument: `A={128,128,128}` vs `B={v,v,0}` for all 256 `v` | [x] |
| 70 | `contrast_ratio` | boundary sweep of the transfer branch point: all `v` in `0..=21` × all `w` in `0..=21` (both colours straddling the `0.04045` threshold, all-channels-equal) | [x] |
| 71 | `contrast_ratio` | high boundary: all `v` in `234..=255` × all `w` in `234..=255` (both colours near max, `pow` arm, ratio ≈ 1) | [x] |
| 72 | `contrast_ratio` | `A == B` for 4096 randomized colours (exact self-division, must be exactly 1.0f) | [x] |
| 73 | `contrast_ratio` | fully randomized full-range: 200000 uniformly random `(A,B)` pairs over all 6 bytes — unbiased cross-check of every branch mask and both swap directions | [x] |
| 74 | `contrast_ratio` | extreme ratio: `A = {0,0,0}` vs `B` = each of the 3 minimal single-channel colours `{1,0,0} {0,1,0} {0,0,1}` and all 255 non-zero greys (largest finite ratios / infinities) | [x] |
| 75 | `contrast_ratio` | argument symmetry: for 50000 randomized pairs, assert `f(A,B)` bits == `f(B,A)` bits in *both* libraries (validates the `High < Low` swap identically) | [x] |
| 76 | `contrast_ratio` | register-padding independence: same 3 payload bytes passed with the upper register bytes varied (struct is 3 bytes, ABI-packed) over 4096 randomized cases | [x] |
