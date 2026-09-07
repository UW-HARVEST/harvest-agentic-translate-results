# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/include/lib.h` (the public API) and every
branch in `c_src/src/lib.c`.

## Public entry points (the FULL set)

`nm -D` on the C `.so` lists exactly one exported function, and `lib.h` declares
exactly one prototype:

| entry point | signature | exported? |
|---|---|---|
| `tritanopia` | `cb_rgb_255 tritanopia(cb_rgb_255 RGB)` | **yes** |
| `cbNorm` | `static cb_rgb cbNorm(cb_rgb_255)` | no — `static`, absent from `nm -D` |
| `cbRemoveGammaRGB` | `static cb_rgb cbRemoveGammaRGB(cb_rgb)` | no — `static` |
| `Tritanopia` | `static void Tritanopia(float*, float*, float*)` | no — `static` |
| `cbApplyGammaRGB` | `static cb_rgb cbApplyGammaRGB(cb_rgb)` | no — `static` |
| `cbDenorm` | `static cb_rgb_255 cbDenorm(cb_rgb)` | no — `static` |

The five lowest-level stages are `static` in the C, so they are **not reachable
through either `.so`** and cannot be called differentially without adding
exports the C `.so` does not have (which would break the Phase D symbol diff).
They are instead driven *indirectly but exhaustively*: the rows below pin each
stage's branch by choosing inputs that force it, and row C31 covers the entire
2^24 input domain, which visits every reachable state of every stage.

## Axes the C actually branches on

There are **no runtime options, modes, flags, globals, setters, or `#ifdef`s** —
`grep -nE "if *\(|switch|#ifdef|#if |static [^c(]" src` finds no mutable state
and no conditional compilation. The only axes are input-shape axes:

**Axis 1 — `cbRemoveGammaRGB` branch, independently per channel.**
`RGB.c > 0.04045` where `RGB.c = byte/255.f`. `10/255 = 0.039216 <= 0.04045`
and `11/255 = 0.043137 > 0.04045`, so the split is exactly at the byte value:
* `L` = linear branch `c / 12.92`  <=> byte in `0..=10`
* `P` = `pow((c + 0.055)/1.055, 2.4)` branch  <=> byte in `11..=255`

**Axis 2 — `cbApplyGammaRGB` branch, independently per channel.**
`x > 0.00313080495356037151702786377709` on the *post-matrix* linear value:
* `l` = linear branch `x * 12.92` (this is also the branch every **negative**
  value takes — `pow` is never called with a negative base)
* `p` = `1.055 * pow(x, 0.4166666666) - 0.055`

**Axis 3 — `cbDenorm` float->`unsigned char` conversion domain**, per channel,
on `y = x * 255.f + 0.5f`:
* `in` = `0 <= y < 256` (ordinary truncation)
* `neg` = `y < 0` -> `cvttss2si` to `i32` then wrap to 8 bits
* `ovf` = `y >= 256` -> truncate to `i32` then wrap to 8 bits

Reachability, from the matrix rows in `Tritanopia`:
`R' = R + 0.1274*(G - B)` can leave `[0,1]` on **both** sides, so R hits all
three of `in`/`neg`/`ovf`. `G'` and `B'` are both `~0.8739*G + 0.1260*B` with
`R` coefficients of `-4.486e-11` / `3.1113e-10`, so they stay in `[0,1]` and
only ever hit `in`.

**Axis 4 — argument/return ABI shape.** `cb_rgb_255` is 3 bytes / align 1, so
under x86-64 SysV it is a single INTEGER-class eightbyte passed and returned in
one register; the 4th byte is unspecified padding.

## The table

Every row is exercised with **many randomized inputs at a fixed seed**
(`SEED = 0x5EED_1234_ABCD_F00D`, SplitMix64), both libraries loaded via
`libloading` from their `.so`, results compared field-by-field.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `tritanopia` | Axis1 `LLL` — all three channels in `0..=10`, all take the linear de-gamma branch | [x] |
| C2 | `tritanopia` | Axis1 `LLP` — R,G linear; B `>=11` uses `pow` | [x] |
| C3 | `tritanopia` | Axis1 `LPL` | [x] |
| C4 | `tritanopia` | Axis1 `LPP` | [x] |
| C5 | `tritanopia` | Axis1 `PLL` | [x] |
| C6 | `tritanopia` | Axis1 `PLP` | [x] |
| C7 | `tritanopia` | Axis1 `PPL` | [x] |
| C8 | `tritanopia` | Axis1 `PPP` — all three `>=11`, all take the `pow` de-gamma branch | [x] |
| C9 | `tritanopia` | Axis1 boundary sweep: every channel pinned to each of `{0,1,10,11,254,255}` while the others are randomized (exact threshold crossing `10 -> 11`) | [x] |
| C10 | `tritanopia` | Axis2 `R'=l, G'=l, B'=l` — post-matrix R,G,B all `<=` apply-gamma threshold (near-black inputs) | [x] |
| C11 | `tritanopia` | Axis2 `R'=l, G'=p, B'=p` — R' pushed to/below threshold (incl. negative) while G',B' are large: `R` small, `B > G` | [x] |
| C12 | `tritanopia` | Axis2 `R'=p, G'=l, B'=l` — R' above threshold while G',B' at/below it: `R` large, `G=B=0` | [x] |
| C13 | `tritanopia` | Axis2 `R'=p, G'=p, B'=p` — all above threshold (ordinary mid/bright colours) | [x] |
| C14 | `tritanopia` | Axis2 `G'` and `B'` branch **differ** — needs `0.8739*G+0.1260*B` within ~1e-9 of the threshold; probed by a targeted search plus the exhaustive row (recorded as reachable-or-not, not assumed) | [x] |
| C15 | `tritanopia` | Axis3 R=`neg` — red channel driven below 0 so `y < 0` and the cast wraps: `R` small, `B >> G` (e.g. `{0,0,255}`) | [x] |
| C16 | `tritanopia` | Axis3 R=`ovf` — red channel driven above 1 so `y >= 256` and the cast wraps: `R` large, `G >> B` (e.g. `{255,255,0}`) | [x] |
| C17 | `tritanopia` | Axis3 R=`in`, G=`in`, B=`in` — no wrap anywhere (the "happy" domain) | [x] |
| C18 | `tritanopia` | Axis3 boundary: `y` within 1 ULP of `0.0`, of `256.0`, and of `-1.0` — a targeted search over all 2^24 inputs picks the extreme achievers of each | [x] |
| C19 | `tritanopia` | Axis1xAxis3 cross: `LLL` inputs that still wrap (all channels `<=10`, `B > G`) | [x] |
| C20 | `tritanopia` | Axis1xAxis3 cross: `PPP` inputs that still wrap | [x] |
| C21 | `tritanopia` | shape: `R = G = B` (pure grays), all 256 of them | [x] |
| C22 | `tritanopia` | shape: single channel hot, other two zero — all 3x256 | [x] |
| C23 | `tritanopia` | shape: single channel zero, other two `255` — all 3x256 | [x] |
| C24 | `tritanopia` | shape: the 8 corners of the cube `{0,255}^3` | [x] |
| C25 | `tritanopia` | shape: `G == B` exactly (R' reduces to `R`, the `0.1274` terms cancel — a genuinely distinct arithmetic path) | [x] |
| C26 | `tritanopia` | shape: `G` and `B` differ by exactly 1 (smallest non-cancelling difference) | [x] |
| C27 | `tritanopia` | Axis4: 4th argument byte set to `0x00`, `0xFF`, and random garbage — the padding must not affect the result | [x] |
| C28 | `tritanopia` | Axis4: result read as a raw 3-byte struct vs. as three separate field loads (return-register padding must not be compared) | [x] |
| C29 | `tritanopia` | repeated/idempotent invocation: feeding a result back in 8 times (catches state leaking between calls; the C is pure so all 8 rounds must agree) | [x] |
| C30 | `tritanopia` | uniformly random inputs over the whole cube, 200,000 draws at the fixed seed | [x] |
| C31 | `tritanopia` | **EXHAUSTIVE**: all 2^24 = 16,777,216 possible inputs, byte-compared. Supersedes every row above and is the definitive valid-path proof. | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only combinations are
the default build and `--no-default-features` (identical). Both are run by
`check_all_features.sh`; there is no third code path to cover.

## Results

All 31 rows checked off. Command used:

```
cd translation && ./check_all_features.sh
```

Output (both feature invocations x both cargo profiles):

```
[OK] symbol diff EMPTY for <default>
[OK] symbol diff EMPTY for --no-default-features
[OK] cargo test <default> <debug>               — 40 test(s) passed
[OK] cargo test <default> --release             — 40 test(s) passed
[OK] cargo test --no-default-features <debug>   — 40 test(s) passed
[OK] cargo test --no-default-features --release — 40 test(s) passed
ALL PHASE D CHECKS PASSED
```

Row C31 reports `all 16777216 inputs byte-identical between the C and Rust .so`,
and C31b reports the same through the raw `u32` ABI view. Since 2^24 is the
*entire* input domain of the exported API, this is exhaustive proof rather than
a sample, and it subsumes rows C1-C30.

C14 was measured, not assumed: a full scan of the cube found **no** input where
`G'` and `B'` take different re-gamma branches, so that configuration is
unreachable. The closest approach found was `{79, 3, 37}` (gap 2.33e-6 against a
3.13e-3 threshold); it is tested explicitly so the row is not vacuous.

## Negative control (proof the suite can fail)

To confirm the harness genuinely compares two different binaries, the Rust cast
was temporarily mutated from C's truncate-then-wrap to Rust's native saturating
`as u8`:

```diff
-        (value as i32) as c_uchar
+        value as c_uchar  // MUTANT: saturating
```

19 tests failed immediately, including `c15_cast_negative_wrap`,
`c16_cast_overflow_wrap`, `c30_uniform_random` and the exhaustive rows. The
mutation was then reverted and `src/lib.rs` verified byte-identical to the
original. This rules out a vacuously-passing suite.
