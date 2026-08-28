# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`, and from an
instrumented enumeration of the **entire** input domain (see "Derivation" below).

## Axis enumeration (what the C actually branches on)

**Runtime options / modes / flags: NONE.** Mechanically:

```sh
grep -nE '#if|#ifdef|switch|extern|global|static [a-z_]+ [A-Za-z_]+ *(=|;)' c_src/src/lib.c
```

...finds no `#ifdef`, no `switch`, no mutable global/static state, no init
function, no context/handle struct, no setter, no flag argument. `include/lib.h`
declares exactly one function and one struct. The library is **stateless and
option-free**: `tritanopia` is a pure function of its 3-byte argument. There is
therefore no option cross-product to take — the configuration surface is entirely
the **input-shape** surface.

**Public entry points (full set, lowest level included):** exactly one,
`tritanopia`. The five lower-level functions (`cbNorm`, `cbRemoveGammaRGB`,
`Tritanopia`, `cbApplyGammaRGB`, `cbDenorm`) are `static` and thus absent from
both `.so`s' dynamic symbol tables (see `SYMBOLS.md`), so they cannot be called
directly by any external consumer. They are exercised **as the composed
pipeline** — which is the stated goal anyway, since composed-pipeline bugs are
invisible to per-wrapper tests. The rows below are the internal branch
combinations of that pipeline, so every internal arm is covered even though only
one symbol is callable.

**Input-shape axes the code special-cases** (the 6 ternaries + 3 conversions):

| axis | values | source |
|------|--------|--------|
| A. `cbRemoveGammaRGB` arm, **per channel** | `0` = linear (`c/12.92`) when `c/255 <= 0.04045`, i.e. byte `<= 10`; `1` = `pow` arm when byte `>= 11` | `lib.c:13-18` |
| B. `cbApplyGammaRGB` arm, **per channel** | `0` = linear (`c*12.92`) when `c <= 0.0031308…`; `1` = `pow` arm | `lib.c:36-44` |
| C. `cbDenorm` float→`u8` conversion bucket, **per channel** | `0` = value `< 0` (UB, wraps); `1` = value in `[0,256)` (normal); `2` = value `>= 256` (UB, wraps) | `lib.c:29-31` |

Axes A, B, C are per-channel and independent in principle → `8 * 8 * 27 = 1728`
candidate combinations. Pruned to what the code **actually distinguishes** by
enumerating all 2^24 inputs: **28 reachable signatures**, listed below. (The
other 1700 are unreachable — e.g. the G and B outputs are always in bucket `1`,
because the tritanopia matrix rows for G and B are convex-ish blends that never
leave range, while the R row `R + 0.1274*G - 0.1274*B` does.)

## Derivation

An instrumented copy of the C (compiled separately under `target/probe/`,
`c_src` untouched) classified every one of the 16,777,216 inputs by its
`(A,B,C)` signature:

```
removeGamma: pow=48168960 lin=2162688        (both arms reachable)
applyGamma : pow=48358988 lin=1972660        (both arms reachable)
cbDenorm arg range: [-419.228302, 269.282959]
cbDenorm buckets  : neg=1660713  in[0,256)=48516762  >=256=154173
distinct signatures = 28
```

Both UB conversion buckets are reachable from ordinary input, so a translation
that clamped or saturated instead of wrapping would be wrong for 1,814,886 of
the 50,331,648 output channels.

## Configuration table

Notation: triples are `R,G,B`. `arm` `0`=linear, `1`=`pow`.
`bucket` `0`=negative→wrap, `1`=in range, `2`=`>=256`→wrap.
"count" = number of the 2^24 inputs with this signature. "example" = smallest
witness found. Each row is tested with **every** input having that signature when
the count is small, and with a seeded random sample plus the witness otherwise;
all 28 are additionally covered by the exhaustive sweep.

| # | entry point(s) | configuration (arms + input shape) | count | example (R,G,B) | [ ] |
|---|----------------|-------------------------------------|-------|-----------------|-----|
| 1 | `tritanopia` | removeGamma arm=000 / applyGamma arm=000 / denorm bucket=011 | 28 | (0,0,4) | [x] |
| 2 | `tritanopia` | removeGamma arm=001 / applyGamma arm=000 / denorm bucket=011 | 743 | (0,0,11) | [x] |
| 3 | `tritanopia` | removeGamma arm=001 / applyGamma arm=011 / denorm bucket=011 | 26476 | (0,0,44) | [x] |
| 4 | `tritanopia` | removeGamma arm=101 / applyGamma arm=011 / denorm bucket=011 | 103612 | (11,0,47) | [x] |
| 5 | `tritanopia` | removeGamma arm=011 / applyGamma arm=011 / denorm bucket=011 | 315650 | (0,11,15) | [x] |
| 6 | `tritanopia` | removeGamma arm=111 / applyGamma arm=011 / denorm bucket=011 | 1214204 | (11,11,50) | [x] |
| 7 | `tritanopia` | removeGamma arm=000 / applyGamma arm=000 / denorm bucket=111 | 1267 | (0,0,0) | [x] |
| 8 | `tritanopia` | removeGamma arm=100 / applyGamma arm=000 / denorm bucket=111 | 15 | (11,0,6) | [x] |
| 9 | `tritanopia` | removeGamma arm=010 / applyGamma arm=000 / denorm bucket=111 | 59 | (0,11,0) | [x] |
| 10 | `tritanopia` | removeGamma arm=001 / applyGamma arm=000 / denorm bucket=111 | 1655 | (0,8,11) | [x] |
| 11 | `tritanopia` | removeGamma arm=101 / applyGamma arm=000 / denorm bucket=111 | 610 | (11,0,11) | [x] |
| 12 | `tritanopia` | removeGamma arm=000 / applyGamma arm=100 / denorm bucket=111 | 36 | (10,3,0) | [x] |
| 13 | `tritanopia` | removeGamma arm=100 / applyGamma arm=100 / denorm bucket=111 | 29630 | (11,0,0) | [x] |
| 14 | `tritanopia` | removeGamma arm=010 / applyGamma arm=100 / denorm bucket=111 | 7 | (9,11,0) | [x] |
| 15 | `tritanopia` | removeGamma arm=110 / applyGamma arm=100 / denorm bucket=111 | 1470 | (11,11,0) | [x] |
| 16 | `tritanopia` | removeGamma arm=101 / applyGamma arm=100 / denorm bucket=111 | 52800 | (11,6,11) | [x] |
| 17 | `tritanopia` | removeGamma arm=010 / applyGamma arm=011 / denorm bucket=111 | 2290 | (0,11,6) | [x] |
| 18 | `tritanopia` | removeGamma arm=001 / applyGamma arm=011 / denorm bucket=111 | 771 | (0,10,13) | [x] |
| 19 | `tritanopia` | removeGamma arm=101 / applyGamma arm=011 / denorm bucket=111 | 7948 | (11,0,44) | [x] |
| 20 | `tritanopia` | removeGamma arm=011 / applyGamma arm=011 / denorm bucket=111 | 28532 | (0,11,11) | [x] |
| 21 | `tritanopia` | removeGamma arm=111 / applyGamma arm=011 / denorm bucket=111 | 92160 | (11,11,16) | [x] |
| 22 | `tritanopia` | removeGamma arm=010 / applyGamma arm=111 / denorm bucket=111 | 27289 | (0,44,0) | [x] |
| 23 | `tritanopia` | removeGamma arm=110 / applyGamma arm=111 / denorm bucket=111 | 646165 | (11,11,6) | [x] |
| 24 | `tritanopia` | removeGamma arm=101 / applyGamma arm=111 / denorm bucket=111 | 495305 | (11,10,13) | [x] |
| 25 | `tritanopia` | removeGamma arm=011 / applyGamma arm=111 / denorm bucket=111 | 316093 | (0,47,11) | [x] |
| 26 | `tritanopia` | removeGamma arm=111 / applyGamma arm=111 / denorm bucket=111 | 13258228 | (11,11,11) | [x] |
| 27 | `tritanopia` | removeGamma arm=110 / applyGamma arm=111 / denorm bucket=**2**11 (R overflows ≥256) | 12640 | (241,253,0) | [x] |
| 28 | `tritanopia` | removeGamma arm=111 / applyGamma arm=111 / denorm bucket=**2**11 (R overflows ≥256) | 141533 | (241,254,11) | [x] |

Rows 1–6 are the negative-wrap family (R bucket `0`); rows 27–28 the
overflow-wrap family (R bucket `2`); rows 7–26 the fully in-range family.

## Additional shape rows (value-dependent / boundary shapes)

Not new branch signatures, but distinct *data shapes* the task calls for
(boundary values, empty/one/many, greys, per-channel isolation). Each is a
separate differential test.

| # | entry point(s) | configuration (input shape) | [ ] |
|---|----------------|------------------------------|-----|
| S1 | `tritanopia` | all 8 corners of the cube `{0,255}^3` | [x] |
| S2 | `tritanopia` | the 4 threshold-boundary bytes `{10,11}` in every channel combination (`{10,11}^3`) — one step either side of axis A's boundary | [x] |
| S3 | `tritanopia` | the 256 greys `R=G=B=v` | [x] |
| S4 | `tritanopia` | single-channel ramps: `(v,0,0)`, `(0,v,0)`, `(0,0,v)` for all 256 `v` | [x] |
| S5 | `tritanopia` | saturated-pair ramps: `(v,255,255)`, `(255,v,255)`, `(255,255,v)` for all 256 `v` | [x] |
| S6 | `tritanopia` | the low band `0..=12` in all channels (`13^3`) — dense coverage of the linear arms | [x] |
| S7 | `tritanopia` | the high band `243..=255` in all channels (`13^3`) — dense coverage of the overflow-wrap region | [x] |
| S8 | `tritanopia` | seeded pseudorandom sample, 400,000 inputs (xorshift64\*, fixed seed) | [x] |
| S9 | `tritanopia` | **exhaustive**: all 16,777,216 inputs, byte-for-byte | [x] |
| S10 | `tritanopia` | repeated/interleaved calls to both `.so`s to prove statelessness (same input yields same output regardless of call order/history) | [x] |
| S11 | `tritanopia` | garbage in the upper register bytes of the by-value struct argument (see `ERRORS.md` row 9) | [x] |

S9 subsumes rows 1–28 and S1–S7 as a mathematical certainty; the individual rows
are kept as separate tests so a failure localises to a specific branch signature
instead of reporting "one of 16.7M inputs differs".

## Result

All 28 signature rows and all 11 shape rows pass, under every configuration
(`dev`/`release` x default/`--no-default-features`). Driven by `../verify.sh`.

Row 1–28 evidence (`configs_rows_1_to_28_all_branch_signatures`): 42,705
differential checks — every stored witness plus 8 seeded random perturbations per
witness. Family breakdown (`configs_denorm_bucket_families`): negative-wrap 1,028
inputs, in-range 3,317, overflow-wrap 400.

S9 evidence: **all 16,777,216 inputs byte-identical**. Because the public API's
entire input domain is a 3-byte struct, S9 is not a sample — it is a complete
proof of behavioural equivalence for every input the library can ever receive.

### Reproducing the generated witness data

`tests/data/signatures.txt` is generated from the C ground truth by
`tests/data/gen_signatures.c`, and the range figures by
`tests/data/probe_ranges.c`. Both `#include "lib.c"` so they can observe the
`static` helpers; they are compiled *outside* `c_src`, which is never modified:

```sh
cd translation/target/probe   # any scratch dir
gcc -O2 -o gen ../../tests/data/gen_signatures.c \
    -I../../../c_src/src -I../../../c_src/include -lm
./gen > ../../tests/data/signatures.txt
```
