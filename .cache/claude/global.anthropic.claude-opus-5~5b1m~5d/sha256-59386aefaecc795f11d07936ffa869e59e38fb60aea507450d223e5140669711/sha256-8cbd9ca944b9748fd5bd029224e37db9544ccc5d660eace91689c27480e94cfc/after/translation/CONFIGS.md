# CONFIGS.md — Configuration-surface table

Mechanically derived from `c_src/include/lib.h` + `c_src/src/lib.c`.

## Axes the C actually branches on

The library has **no** runtime options, no init/context struct, no mode flags,
no byte-order or element-type parameters, and no `#ifdef` in the source:

```
$ grep -cE '#if|#ifdef|#ifndef' c_src/src/lib.c
0
$ grep -nE 'switch|enum|struct|typedef' c_src/src/lib.c
(no matches)
```

`Cargo.toml` declares **no `[features]` table**, so the only feature
combination is the default (empty) one. Confirmed:

```
$ grep -A5 '\[features\]' translation/Cargo.toml
(no matches)
```

So the entire configuration surface is the *shape and values* of the single
3-float input array. The axes the code distinguishes are:

- **A1 — early-out disjunct** (`if (delta == 0 || max == 0)`, line 19):
  neither / `delta == 0` only / `max == 0` only / both.
- **A2 — hue branch selection** (`if (r == max) / else if (g == max) / else`,
  lines 26-31): `r`-branch, `g`-branch, `b`-(else)-branch, and **ties**
  (which resolve to the earliest matching branch).
- **A3 — hue wrap correction** (`if (h < 0)`, line 33): taken / not taken.
- **A4 — value class of each component**: normal positive, zero, `-0.0`,
  negative, subnormal, `FLT_MAX`/`FLT_MIN`, `±INFINITY`, `NaN`.
- **A5 — magnitude / precision regime**: values in `[0,1]` (the documented
  use), large values, tiny values, and mixed magnitudes where `delta`
  underflows or `delta/max` overflows.
- **A6 — pointer aliasing of the two arguments** (the API takes two raw
  pointers and the C has no `restrict`): disjoint, `dest == src`,
  and partial overlap in both directions.

There is exactly **one public entry point**, `rgb_to_hsv`, and it is also the
lowest-level one — there are no convenience wrappers layered on top, so the
"call the low-level API directly" requirement is satisfied by construction.

## Configuration table

Each row is exercised with **many randomized inputs** (fixed seed, a
deterministic SplitMix64/xorshift PRNG in the test file) except where the row
names a specific finite set of values, in which case that whole set is
enumerated exhaustively.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `rgb_to_hsv` | A5 `[0,1]` random chromatic triples, disjoint pointers — the documented happy path | [x] |
| 2  | `rgb_to_hsv` | A2 `r`-branch: `r` strictly the unique max, random `[0,1]` | [x] |
| 3  | `rgb_to_hsv` | A2 `g`-branch: `g` strictly the unique max, random `[0,1]` | [x] |
| 4  | `rgb_to_hsv` | A2 `b`-branch (`else`): `b` strictly the unique max, random `[0,1]` | [x] |
| 5  | `rgb_to_hsv` | A2 tie `r == g > b`: first-match (`r`) branch must win | [x] |
| 6  | `rgb_to_hsv` | A2 tie `g == b > r`: `g`-branch must win over `else` | [x] |
| 7  | `rgb_to_hsv` | A2 tie `r == b > g`: `r`-branch must win | [x] |
| 8  | `rgb_to_hsv` | A3 wrap taken: `r == max` and `g < b`, so `h` negative → `+360` | [x] |
| 9  | `rgb_to_hsv` | A3 wrap not taken: `r == max` and `g >= b` | [x] |
| 10 | `rgb_to_hsv` | A1 `delta == 0` via `r == g == b`, random non-zero grey | [x] |
| 11 | `rgb_to_hsv` | A1 both disjuncts: exact `{0,0,0}` | [x] |
| 12 | `rgb_to_hsv` | A1 `max == 0` only: all `<= 0`, at least one negative, max exactly `+0.0` | [x] |
| 13 | `rgb_to_hsv` | A1 neither, but `max < 0`: all components negative → negative `s`/`v` | [x] |
| 14 | `rgb_to_hsv` | A4 `-0.0` in every position and combination (exhaustive over the 8 sign patterns of `{±0.0}^3`) | [x] |
| 15 | `rgb_to_hsv` | A4 single `NaN` in each of the 3 positions, other two random finite | [x] |
| 16 | `rgb_to_hsv` | A4 two `NaN`s, all 3 position pairs | [x] |
| 17 | `rgb_to_hsv` | A4 all three `NaN` | [x] |
| 18 | `rgb_to_hsv` | A4 `+INFINITY` in each position, others random finite | [x] |
| 19 | `rgb_to_hsv` | A4 `-INFINITY` in each position, others random finite | [x] |
| 20 | `rgb_to_hsv` | A4 mixed `+INFINITY` and `-INFINITY` in the same triple | [x] |
| 21 | `rgb_to_hsv` | A4 subnormal components (random subnormal bit patterns) — `delta` may underflow to `0` | [x] |
| 22 | `rgb_to_hsv` | A5 `FLT_MAX` / `FLT_MIN` / `FLT_EPSILON` boundary triples (exhaustive over the 27 combinations) | [x] |
| 23 | `rgb_to_hsv` | A5 huge magnitudes (`1e30`-scale) where `max - min` overflows to `+inf` | [x] |
| 24 | `rgb_to_hsv` | A5 mixed magnitude: one huge, one tiny, one normal | [x] |
| 25 | `rgb_to_hsv` | A4/A5 fully unconstrained random 32-bit patterns reinterpreted as `f32` (covers all classes incl. signalling NaN payloads) | [x] |
| 26 | `rgb_to_hsv` | A6 `dest == src` full aliasing, random chromatic input | [x] |
| 27 | `rgb_to_hsv` | A6 partial overlap `dest == src + 1` within a 4-float buffer | [x] |
| 28 | `rgb_to_hsv` | A6 partial overlap `dest == src - 1` within a 4-float buffer | [x] |
| 29 | `rgb_to_hsv` | A6 `src` pointing into the middle of a larger buffer (offset, unaligned-by-index reads) with a random tail | [x] |
| 30 | `rgb_to_hsv` | A2+A3+A1 cross-product sweep: quantized grid over `{0, 0.5, 1, -1, 2}^3` (exhaustive, 125 triples) hitting every branch combination the code distinguishes | [x] |

## Notes recorded while running Phase B

**Rows 26-29 initially had a real blind spot.** They drove aliasing with random
*chromatic* inputs only, so the `delta == 0 || max == 0` **early-out** path was
never exercised while `dest` overlapped `src`. A mistranslation that re-reads
`src` after storing into `dest` is invisible unless BOTH conditions hold at once.
`mutation_check.sh` caught this (the mutant "early-out re-reads src after
writing dest[0]" survived), and rows 26-29 plus `ERRORS.md` rows 13-14 now fold
in `early_out_triples()`: black, signed-zero black, positive grey, **negative**
grey, `FLT_MIN` grey, and the `max == 0 && delta != 0` shapes, alongside ~2000
randomized greys / `max == 0` triples. This is the interaction-of-axes class of
bug `CONFIGS.md` exists to find.

**Harness pitfall fixed:** `cargo test` does **not** rebuild a
`crate-type = ["cdylib"]` library target, so plain `cargo test` dlopens a stale
`.so` and every differential test passes vacuously. Both test files now call
`assert_so_is_fresh()` before loading, and `run_tests.sh` builds the cdylib
before each test invocation and passes its path via `RUST_SO_PATH`.

## Verification of the tests themselves

`mutation_check.sh` injects 34 plausible mistranslations into `src/lib.rs`,
rebuilds, and re-runs the suite. Result: **28/28 behaviour-changing mutants
killed**, and 6 mutants that are *provably observationally equivalent* to the C
correctly survive (each with its proof inline in the script):

| mutant | why it is genuinely equivalent |
|--------|--------------------------------|
| `c_min` uses `<=` | differs only for `{+0.0, -0.0}`; `min` feeds only `delta`, and every such case is swallowed by the early-out |
| branch order `g` before `r` | branches can only disagree on a max-tie, where the formulae coincide exactly (`delta/delta == 1` vs `2 + (-delta)/delta == 1`) |
| branch order else-first | same tie-invariance argument |
| `delta` computed in f64 | Figueroa: double rounding is innocuous for one op when `53 >= 2*24 + 2` |
| `s = delta/max` computed in f64 | same theorem, single widened division |
| wrap via `rem_euclid(360)` | `\|g-b\| <= delta` bounds `h` to `[-60, 300] u {NaN}`, so it can never overflow and `rem_euclid` performs the identical `h + 360.0` |

Crucially, widening a *chain* is NOT equivalent (the C rounds the intermediate
`g - b` to f32) and those mutants ARE killed, as are all sign, constant, index,
comparison-direction, NaN-normalisation, output-clamping and aliasing mutants.

No binary/driver executable is built: `c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)` and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout" gate
is therefore not applicable.
