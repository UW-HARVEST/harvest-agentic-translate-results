# CONFIGS.md — configuration-surface table (Phase A, gates Phase B)

## Mechanical derivation

`c_src/include/lib.h` declares the **entire** public API — one entry point,
which is also the lowest-level entry point (there is no convenience wrapper and
no internal helper to reach past):

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There is **no** runtime option struct, no mode/flag argument, no global state,
no `#ifdef`, and no `switch`. So the configuration axes are exactly the ones the
body branches or computes on:

| axis | where the C branches / varies on it | distinct values the C treats differently |
|------|-------------------------------------|------------------------------------------|
| `size` **sign** | `hsize = size/2` (line 10) feeds loop guard `-hsize <= hsize` (15) and `r < size` (25) | `size <= -2` (no iterations) · `size ∈ {-1,0}` (1 iteration, no normalisation) · `size >= 1` (normal) |
| `size` **parity** | `2*hsize+1` vs `size` (lines 15 vs 25) | odd (`2*hsize+1 == size`, fully normalised) · even (`2*hsize+1 == size+1`, one element written but **not** normalised) |
| `size` **magnitude** | trip count of both loops | 1 · 2 · 3 · 4 · 5 · small (≤ 33) · large (hundreds/thousands, exercises `sum` accumulation order) |
| `radius` **class** | `rs = sigma/radius` (12), then `x*x` (17) | normal positive · normal negative · `±0.0` · `±inf` · `NaN` · subnormal · overflowing-`rs` · underflowing-`rs` |
| `radius` **vs `hsize`** | decides how many `r` hit the clamp `v > 0` (18) | all clamped (`sum == 0`, no normalisation) · some clamped (mixed) · none clamped (flat/wide kernel) |
| clamp branch (18) | `((v) > (0)) ? (v) : (0)` | taken (`v > 0`) · not taken (`v < 0`) · not taken because `v` is `NaN` · not taken because `v == 0` exactly |
| normalise branch (23) | `if (sum > 0.0f)` | true · false |
| destination buffer | unchecked stores at `dest[0 .. 2*hsize]` | in-bounds region · the one-past-the-end element for even `size` · guard bytes that must stay untouched |

Rows below are the pruned cross-product: one row per combination the C actually
distinguishes. Every row is driven **through the `.so` export of both C and
Rust** with **many randomized inputs** (fixed seed `0x5EED_1234`, a
deterministic SplitMix64/xorshift PRNG defined in `tests/common/mod.rs`), and
the whole destination buffer — including padding/guard elements — is compared
**bitwise** (`f32::to_bits`) so `-0.0` vs `+0.0`, `NaN` payloads, and untouched
guard bytes all count as divergences.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `gaussian_kernel` | `size == 1` (odd, minimal), `radius` randomized over normal positives in `[1e-3, 1e3]` | [x] |
| C2 | `gaussian_kernel` | `size == 2` (even, minimal ⇒ 3 stores, 2 normalised), `radius` randomized normal positive | [x] |
| C3 | `gaussian_kernel` | `size == 3` (odd, smallest with a non-centre tap), `radius` randomized normal positive | [x] |
| C4 | `gaussian_kernel` | `size == 4` (even), `radius` randomized normal positive | [x] |
| C5 | `gaussian_kernel` | `size == 5` (odd), `radius` randomized normal positive | [x] |
| C6 | `gaussian_kernel` | odd `size` randomized in `[7, 33]`, `radius` randomized normal positive — mixed clamped/unclamped taps | [x] |
| C7 | `gaussian_kernel` | even `size` randomized in `[6, 32]`, `radius` randomized normal positive — mixed taps **plus** the one-past-the-end store | [x] |
| C8 | `gaussian_kernel` | odd `size` randomized in `[101, 1025]` (large), `radius` randomized normal positive — long `sum` accumulation chain, float add order matters | [x] |
| C9 | `gaussian_kernel` | even `size` randomized in `[100, 1024]` (large), `radius` randomized normal positive | [x] |
| C10 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius` **negative** randomized in `[-1e3, -1e-3]` (`rs < 0`, `x*x` even) | [x] |
| C11 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius == +0.0f` ⇒ `rs = +inf` ⇒ all taps `0.0f`, `sum == 0`, normalisation **skipped** | [x] |
| C12 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius == -0.0f` ⇒ `rs = -inf`, same all-zero unnormalised result | [x] |
| C13 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius == f32::INFINITY` ⇒ `rs = +0.0` ⇒ **flat** kernel, normalised by `1/(2*hsize+1)` | [x] |
| C14 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius == f32::NEG_INFINITY` ⇒ `rs = -0.0` ⇒ flat kernel, `x = -0.0*r` | [x] |
| C15 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius == f32::NAN` ⇒ every tap clamped from `NaN` to `0.0f`, normalisation skipped | [x] |
| C16 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius` **subnormal** (randomized bit patterns in `[1, 0x007F_FFFF]`) ⇒ `rs` overflows to `+inf` | [x] |
| C17 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius` **huge finite** (randomized in `[1e20, 1e38]`) ⇒ `rs` underflows to subnormal/zero ⇒ flat kernel | [x] |
| C18 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius` **tiny normal** (randomized in `[1e-38, 1e-6]`) ⇒ Dirac-spike or all-zero regime boundary | [x] |
| C19 | `gaussian_kernel` | `size` randomized `[3, 65]`, `radius` tuned so `\|r\|*rs` lands **exactly** on the clamp boundary `2.4` for some integer `r` (`radius = 1.5*r`, randomized `r`) ⇒ exercises `v == 0.0f` exactly with the strict `>` | [x] |
| C20 | `gaussian_kernel` | `size` randomized `[1, 65]`, `radius` = fully **randomized raw `f32` bit patterns** (any class: normal/subnormal/inf/NaN, either sign) — unconstrained property test | [x] |
| C21 | `gaussian_kernel` | `size == 0` ⇒ one store at `dest[0]`, no normalisation; `radius` randomized over all classes | [x] |
| C22 | `gaussian_kernel` | `size == -1` (truncating division ⇒ `hsize == 0` ⇒ one store); `radius` randomized | [x] |
| C23 | `gaussian_kernel` | `size` randomized in `[-4096, -2]` ⇒ **zero** stores, buffer must be left byte-identical to its pre-call fill; `radius` randomized | [x] |
| C24 | `gaussian_kernel` | `size ∈ {INT_MIN, INT_MIN+1, INT_MIN+2, INT_MAX/2-ish negatives}` extreme negatives ⇒ zero stores, no overflow trap on `-hsize` | [x] |
| C25 | `gaussian_kernel` | pre-filled destination buffer with **non-zero garbage** (randomized bit patterns incl. `NaN`/`inf`) so that "left untouched" and "overwritten" are distinguishable, over randomized `size` `[-4, 65]` × randomized `radius` | [x] |
| C26 | `gaussian_kernel` | **unaligned-tail / guard-region** shape: buffer padded with `PAD` guard elements after the writable region; asserts the one-past-the-end store for even `size` hits exactly `dest[size]` and no further, for randomized `size`/`radius` | [x] |
| C27 | `gaussian_kernel` | **repeated invocation on the same buffer** (call twice back-to-back with different `size`/`radius`) — checks there is no hidden state and that partial overwrite of a previous kernel matches | [x] |
| C28 | `gaussian_kernel` | full randomized cross-product sweep: `size` ∈ randomized `[-8, 129]` × `radius` ∈ randomized raw bit patterns, 20 000 cases, bitwise buffer compare | [x] |

**Total: 28 rows.** All exercised by `translation/tests/configs.rs`.

## Results

All 28 rows pass (`translation/tests/configs.rs`, tests `c01..c28`), under both
the release and the debug Rust `.so`, and against the C rebuilt at `-O0`, `-O2`
and `-O3`. Roughly 30 000 randomized differential cases run in total, each
comparing the entire destination buffer plus its guard region bitwise.

### Suite detection power (mutation testing)

A differential suite that cannot fail proves nothing, so
`translation/mutation_test.sh` injects deliberate bugs into
`translation/src/lib.rs` and requires the suite to catch them:

| injected bug | outcome |
|--------------|---------|
| floor division instead of C truncation for `size / 2` | **caught** |
| exclusive loop bound (`r < hsize`) | **caught** |
| clamp `else` arm yields `-0.0` instead of `0` | **caught** |
| normalise when `sum >= 0` instead of `sum > 0` | **caught** |
| normalise `size + 1` elements (include the overrun element) | **caught** |
| defensive null check added (survives where C faults) | **caught** |
| `*p /= sum` instead of `*p *= 1.0/sum` | **caught** |
| `f64` intermediate precision | **caught** |
| `sigma` 1.6 → 1.60001 | **caught** |
| `tetha` 2.25 → 2.2499 | **caught** |
| `f32::exp()` instead of extern `expf` | survived — genuinely equivalent: rustc lowers it to the same `expf@GLIBC_2.27` (confirmed with `nm -D` on the mutant `.so`) |
| `k.offset(1)` instead of `k.add(1)` | survived — semantics-preserving |
| clamp `>=` instead of `>` | survived — `v` is provably never `-0.0`, so equivalent |

11 of 11 behaviour-changing mutations are caught; the 3 survivors are each
verified semantics-preserving rather than blind spots.
