# CONFIGS.md — Phase B: configuration-surface table

Derived mechanically from `c_src/src/lib.c`, `c_src/include/lib.h` and
`c_src/CMakeLists.txt`.

## Build-time configuration axes

| source | axes found | conclusion |
|--------|-----------|------------|
| `Cargo.toml` | **no `[features]` section**, no `cfg(feature = ...)` anywhere in `src/` | exactly **one** feature combination exists: the empty set (`--no-default-features`) |
| `c_src/CMakeLists.txt` | no `option()`, no `if()`, no `target_compile_definitions`, no `add_definitions`; unconditionally builds `src/lib.c` and links `m` | one C configuration |
| `c_src/src/lib.c` | **no `#if` / `#ifdef` / `#define`** at all (grep confirms) | no preprocessor variants |

=> The full feature matrix is `{ }` (one combination). Every row below is
therefore run under `cargo test --no-default-features`, and additionally against
both the **debug** and **release** builds of the Rust `cdylib` (optimisation
level is the only remaining build-time axis that can perturb float codegen).

## Runtime configuration axes

`hsl_to_rgb(float *dest, const float *src)` takes no flags, no mode enum, no
context/handle and no length. There is exactly one public entry point (it *is*
the lowest-level entry point — there is no convenience wrapper layer to skip).
The configuration space is therefore entirely the **shape/class of the input
data** plus the **pointer relationship** between `dest` and `src`:

* **axis H — the `h` value class**, because the code branches on it six times:
  `[0,60)`, `[60,120)`, `[120,180)` (falls to `else`, see the `h < 120` typo),
  `[180,240)`, `[240,300)`, `[300,360)`, `>= 360`, `< 0`, `±inf`, `NaN`,
  and each half-open boundary `0/60/120/180/240/300/360` ± 1 ULP.
* **axis S — the `s` value class**, because of the `s == 0` early-out:
  `+0.0`, `-0.0`, subnormal, `(0,1)`, `1.0`, `> 1`, `< 0`, `±inf`, `NaN`.
* **axis L — the `l` value class**, because it drives `c = (1-|2l-1|)·s` and
  `m = l - 0.5c`: `0.0`, `-0.0`, `0.5` (where `|2l-1| = 0`), `1.0`, `(0,1)`,
  `< 0`, `> 1`, subnormal, `±inf`, `NaN`.
* **axis P — the pointer relationship**: disjoint, `dest == src` (in-place),
  `dest = src+1`, `dest = src+2`, `dest = src-1` (partial overlap), and
  4-byte-aligned vs. deliberately mis-aligned buffers.
* **axis B — the raw bit pattern space**: every `u32` is a valid `float`, so a
  row that says "random" samples raw `u32` patterns, not just "plausible"
  colours.

Rows are the cross-product of H × S × L × P pruned to the combinations the C
actually distinguishes. Every row is checked with **many randomized inputs**
from a fixed-seed SplitMix64 PRNG (reproducible), and results are compared as
raw `u32` bit patterns for all three components (so `+0.0` vs `-0.0` and NaN
sign/payload are significant).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `hsl_to_rgb` | H = random in `[0,60)`, S = random in `(0,1]`, L = random in `(0,1)`, P = disjoint — sector-1 body `(c+m, x+m, m)` | [x] |
| 2  | `hsl_to_rgb` | H = random in `[60,120)`, S ∈ `(0,1]`, L ∈ `(0,1)`, P = disjoint — sector-2 body `(x+m, c+m, m)` | [x] |
| 3  | `hsl_to_rgb` | H = random in `[120,180)`, S ∈ `(0,1]`, L ∈ `(0,1)` — hits the **final `else`** because the guard reads `h < 120`; must yield `(m,m,m)` | [x] |
| 4  | `hsl_to_rgb` | H = random in `[180,240)`, S ∈ `(0,1]`, L ∈ `(0,1)` — sector-4 body `(m, x+m, c+m)` | [x] |
| 5  | `hsl_to_rgb` | H = random in `[240,300)`, S ∈ `(0,1]`, L ∈ `(0,1)` — sector-5 body `(x+m, m, c+m)` | [x] |
| 6  | `hsl_to_rgb` | H = random in `[300,360)`, S ∈ `(0,1]`, L ∈ `(0,1)` — sector-6 body `(c+m, m, x+m)` | [x] |
| 7  | `hsl_to_rgb` | H = random in `[360, 1e9)` and `[1e9, f32::MAX]`, S ∈ `(0,1]`, L ∈ `(0,1)` — wrap-around **not** performed, final `else` | [x] |
| 8  | `hsl_to_rgb` | H = random strictly negative, `(-1e9, -0)` and `[-f32::MAX, -1e9]`, S ∈ `(0,1]`, L ∈ `(0,1)` — third branch taken (`h<120 && h<180`), body `(m, c+m, x+m)`, with `fmodf` of a negative quotient | [x] |
| 9  | `hsl_to_rgb` | H = each exact boundary `{0, 60, 120, 180, 240, 300, 360}` × S ∈ `(0,1]` random × L ∈ `(0,1)` random — half-open guard selection | [x] |
| 10 | `hsl_to_rgb` | H = `nextafter(b, ∓inf)` and `nextafter(b, ±inf)` for every boundary `b` (1 ULP either side) × random S, L | [x] |
| 11 | `hsl_to_rgb` | S = `+0.0` (early-out) × H random over the whole float range × L random over the whole float range — `dest[0..3] = l` verbatim, incl. `l = -0.0`/NaN/±inf | [x] |
| 12 | `hsl_to_rgb` | S = `-0.0` (early-out, `-0.0 == 0` is true) × H, L random over the whole range | [x] |
| 13 | `hsl_to_rgb` | S = smallest subnormal `1e-45` and `f32::MIN_POSITIVE/2` (non-zero ⇒ **no** early-out) × H random in every sector × L ∈ `(0,1)` | [x] |
| 14 | `hsl_to_rgb` | S = `1.0` exactly × L = `0.0`, `-0.0`, `0.5`, `1.0` exactly × H over all sectors — the `|2l-1| = 1` / `= 0` corner cases (`c = 0` and `c = s`) | [x] |
| 15 | `hsl_to_rgb` | S random `> 1` (up to `f32::MAX`) × L ∈ `(0,1)` × H over all sectors — no clamping | [x] |
| 16 | `hsl_to_rgb` | S random `< 0` × L ∈ `(0,1)` × H over all sectors — negative saturation, `c < 0` | [x] |
| 17 | `hsl_to_rgb` | L random `< 0` and `> 1` (finite, up to ±`f32::MAX`) × S ∈ `(0,1]` × H over all sectors — `1-\|2l-1\|` large negative | [x] |
| 18 | `hsl_to_rgb` | L = `0.5` exactly (so `2l-1 = 0`, `c = s`, `m = 0.5 - 0.5s`) × S random `(0,1]` × H over all sectors | [x] |
| 19 | `hsl_to_rgb` | L = subnormal, `±f32::MIN_POSITIVE`, `±1e-45` × S ∈ `(0,1]` × H over all sectors — gradual underflow, no FTZ | [x] |
| 20 | `hsl_to_rgb` | H = `±inf` × S ∈ `{+0.0, -0.0, 1.0, random(0,1], NaN}` × L ∈ `{0,0.5,1,random,NaN,±inf}` — `fmodf(±inf,2)` domain path; `-inf` reaches the third branch and *uses* `x` | [x] |
| 21 | `hsl_to_rgb` | S = `±inf` × L over all classes × H over all sectors — `0·inf` NaN generation inside `c` | [x] |
| 22 | `hsl_to_rgb` | L = `±inf` × S ∈ `(0,1]` and `S = ±inf` × H over all sectors — `inf - inf` NaN inside `m` | [x] |
| 23 | `hsl_to_rgb` | H = NaN (random payloads, both signs, quiet **and** signalling) × S, L random — final `else`, `dest = (m,m,m)` | [x] |
| 24 | `hsl_to_rgb` | S = NaN (random payloads, both signs, quiet and signalling) × H over all sectors × L random — early-out **not** taken, NaN propagates through `c`,`m`,`x` | [x] |
| 25 | `hsl_to_rgb` | L = NaN (random payloads, both signs, quiet and signalling) × S ∈ `(0,1]` × H over all sectors | [x] |
| 26 | `hsl_to_rgb` | two of `{h,s,l}` NaN with *different* payloads, and all three NaN — pins down which operand's NaN survives each `addss`/`subss`/`mulss`/`divss` (destination-operand rule) | [x] |
| 27 | `hsl_to_rgb` | **fully random raw `u32` bit patterns** for all of `h`, `s`, `l` (100 000 samples, fixed seed) — the unbiased sweep that no hand-picked row can replace | [x] |
| 28 | `hsl_to_rgb` | random `h`, `s`, `l` drawn from a pool of "interesting" floats (0, ±0, ±1, ±0.5, boundaries, ±inf, NaNs, subnormals, `f32::MAX/MIN`, ULP neighbours) — exhaustive-ish cross-product of edge values | [x] |
| 29 | `hsl_to_rgb` | P = `dest == src` (in-place), over all H sectors and the S-early-out path, random values | [x] |
| 30 | `hsl_to_rgb` | P = partial overlap `dest = src+1`, `dest = src+2`, `dest = src-1`, `dest = src-2`, random values — loads all precede stores | [x] |
| 31 | `hsl_to_rgb` | P = mis-aligned `dest`/`src` (offset 1,2,3 bytes into a byte buffer), random values — `movss` / unaligned read path | [x] |
| 32 | `hsl_to_rgb` | guard-word buffers: assert neither library writes `dest[3]`/`dest[-1]` nor mutates `src[3]`/`src[-1]`, over random values in every sector | [x] |
| 33 | `hsl_to_rgb` | repeated / stateful invocation: 10 000 back-to-back calls interleaving C and Rust on the same buffers, checking there is no hidden global state and no MXCSR (rounding-mode / FTZ) leakage between the two libraries | [x] |
| 34 | `hsl_to_rgb` | dense deterministic sweep of `h` over `[-720, 1080]` in 0.25° steps × fixed `s`,`l` — catches any off-by-one-ULP sector boundary drift | [x] |

## Checklist

- [x] Every row above passes across its randomized inputs, for the single
      feature combination `{}` and for **both** the debug and release builds of
      the Rust `cdylib`.

## Mutation testing — evidence that the suite would catch a divergence

Passing tests only mean something if they can fail. Seven behaviour-changing
mutations were applied to `src/lib.rs`, the `cdylib`s rebuilt, and the full suite
re-run for each:

| mutation | result |
|----------|--------|
| `x = c * (…)` operand order swapped (`ss_mul(paren, c)` → `ss_mul(c, paren)`) | **KILLED** |
| `c = (…) * s` operand order swapped (`ss_mul(paren, s)` → `ss_mul(s, paren)`) | **KILLED** |
| the C bug "fixed": `h < 120.0` → `h >= 120.0` in the third guard | **KILLED** |
| `ss_add(c, m)` → `ss_add(m, c)` in sector 1 | **KILLED** |
| `s == 0.0` → `s == 0.0 && s.is_sign_positive()` (drops the `-0.0` early-out) | **KILLED** |
| `ss_mul(c, 0.5)` → `ss_mul(0.5, c)` | survived — **provably equivalent** |
| `ss_add(l, l)` → `ss_mul(2.0, l)` | survived — **provably equivalent** |

The two survivors are not gaps. `0.5` and `2.0` are never NaN, so the
destination-operand NaN rule in `ss_mul` fires on the same operand either way,
and `l + l ≡ 2.0 * l` and `c * 0.5 ≡ 0.5 * c` are bit-exact for every one of the
2^32 `f32` values (the exponent shifts by one, the mantissa is untouched, and
overflow/underflow behave identically). Every mutation that *is* observable was
detected.

## Build-time verification driver

`./verify.sh` extracts the `[features]` table from `Cargo.toml`, builds its power
set, and for each combination runs `cargo check`, `cargo build --release`, the
`nm -D` symbol diff against the C `.so`, and `cargo test`. With no `[features]`
table it reports `1 combination` and passes it.
