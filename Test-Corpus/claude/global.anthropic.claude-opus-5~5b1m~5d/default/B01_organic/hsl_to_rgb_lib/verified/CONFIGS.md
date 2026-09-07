# CONFIGS.md — configuration-surface table

## Axes mechanically derived from the C source

`c_src/include/lib.h` declares exactly one entry point, and it is also the
lowest-level one — there is no convenience wrapper, no init/context object, no
setter, and no one-shot vs. streaming split:

```c
void hsl_to_rgb(float *dest, const float *src);
```

**Runtime options / modes / flags:** none. There is no option struct, no flag
argument, no global, no `#ifdef` and no `switch` anywhere in `c_src/src/lib.c`.
The *only* thing that steers control flow is the **data** in `src[0..3]`.
Therefore the configuration surface is the cross-product of the input-shape axes
below.

**Cargo features:** `translation/Cargo.toml` declares no `[features]` table, so
the only build configuration is the default (= `--no-default-features`, which is
identical). See the "feature combinations" note at the bottom.

### Axis H — the hue `src[0]`, i.e. which arm of the `if`/`else if` chain runs

Taken verbatim from lines 19–47. Note line 27 tests `h < 120.0f` **twice**
instead of `h >= 120.0f`, so the nominal `[120,180)` arm is *unreachable* and
that arm is instead the one that catches all **negative** hues.

| id | condition in C | arm taken | `dest` formula |
|----|----------------|-----------|----------------|
| H1 | `h >= 0 && h < 60` | line 19 | `c+m, x+m, m` |
| H2 | `h >= 60 && h < 120` | line 23 | `x+m, c+m, m` |
| H3 | `h < 0` (the `h<120 && h<180` quirk) | line 27 | `m, c+m, x+m` |
| H4 | `h >= 120 && h < 180` | **unreachable** — line 27 rejects it, line 31 needs `h>=180` → falls to line 43 | `m, m, m` |
| H5 | `h >= 180 && h < 240` | line 31 | `m, x+m, c+m` |
| H6 | `h >= 240 && h < 300` | line 35 | `x+m, m, c+m` |
| H7 | `h >= 300 && h < 360` | line 39 | `c+m, m, x+m` |
| H8 | `h >= 360`, `h = +INF` | line 43 `else` | `m, m, m` |
| H9 | `h` is NaN (all comparisons false) | line 43 `else` | `m, m, m` |
| H10 | `h = -INF` | line 27 quirk | `m, c+m, x+m` |

### Axis S — the saturation `src[1]`, i.e. the line-10 short-circuit

| id | shape | effect |
|----|-------|--------|
| S0 | `s == 0.0f` or `s == -0.0f` | early return, `dest = l,l,l`, hue ignored entirely |
| S1 | `0 < s <= 1` (normal) | full formula |
| S2 | `s > 1` (out of nominal range, unchecked) | full formula, `c` may exceed 1 |
| S3 | `s < 0` (out of nominal range, unchecked) | full formula, `c` negative |
| S4 | `s` subnormal non-zero (`±1e-45`) | full formula (no short-circuit) |
| S5 | `s = ±INF` | full formula; `c = ±INF`, or NaN when `1-\|2l-1\| == 0` |
| S6 | `s` NaN | full formula (`NaN == 0` is false) |

### Axis L — the lightness `src[2]`, i.e. the `fabsf(2l-1)` chroma shape

| id | shape | effect |
|----|-------|--------|
| L1 | `l < 0.5` | `2l-1 < 0`, `fabsf` flips the sign |
| L2 | `l == 0.5` | `2l-1 == 0`, `c == s` exactly (max chroma) |
| L3 | `l > 0.5` | `2l-1 > 0`, `fabsf` is the identity |
| L4 | `l == 0` / `l == 1` | `c == 0` → `x == 0`, `m == l` (grey edges) |
| L5 | `l` outside `[0,1]` (unchecked) | `1-\|2l-1\|` negative → `c` sign flips |
| L6 | `l = ±INF` | `2l-1 = ±INF`, `c = -INF*s`, `m = ∓INF` |
| L7 | `l` NaN | NaN through `c`, `m`, `x` |

### Axis X — the `fmodf(h/60, 2)` sub-shape inside `x`

| id | shape | why the C distinguishes it |
|----|-------|----------------------------|
| X1 | `h/60` in `[0,2)` — `fmodf` is the identity | `\|fmod-1\|` ramps down then up |
| X2 | `h/60 >= 2` — `fmodf` wraps | the sawtooth repeats every 120° of hue |
| X3 | `h/60 < 0` — `fmodf` keeps the **sign of the dividend** | `fmod-1 ∈ (-3,-1]` → `1-\|·\|` ≤ 0, so `x` has the opposite sign |
| X4 | `h/60` an exact multiple of 2 (`h = 0, 120, 240, 720`) | `fmodf` returns `±0`, `x = c*(1-1) = 0` |
| X5 | `h = ±INF` → `h/60 = ±INF` → `fmodf(±INF, 2)` = NaN | `x` becomes NaN even though `c`/`m` are finite |
| X6 | `h` NaN → `fmodf(NaN,2)` = NaN | `x` NaN (but the `else` arm stores `m` only, so `x` is discarded) |

### Axis A — pointer/buffer shape (the only non-value axis)

| id | shape |
|----|-------|
| A1 | disjoint `dest` and `src` buffers |
| A2 | `dest == src` (full aliasing) |
| A3 | `dest` and `src` partially overlapping (`dest == src+1`, `dest == src-1`) |
| A4 | unaligned (byte-offset) `dest`/`src` inside a larger allocation |

## Configuration table — one row per combination the C treats differently

Every row is exercised against **both** `.so` files with a fixed-seed
(`seed = 0x5EED_1234`) xorshift PRNG generating many inputs per row, and the
three output floats are compared **bit-for-bit** (`to_bits()`), so `+0.0` vs
`-0.0` and NaN payload differences are caught.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsl_to_rgb` | H1 `h∈[0,60)` × S1 × L1 — randomized | [x] |
| 2 | `hsl_to_rgb` | H1 × S1 × L2 (`l = 0.5`, max chroma) — randomized `h`,`s` | [x] |
| 3 | `hsl_to_rgb` | H1 × S1 × L3 — randomized | [x] |
| 4 | `hsl_to_rgb` | H2 `h∈[60,120)` × S1 × L1/L2/L3 — randomized | [x] |
| 5 | `hsl_to_rgb` | H3 `h<0` quirk arm × S1 × L1/L2/L3 — randomized (also exercises X3) | [x] |
| 6 | `hsl_to_rgb` | H4 `h∈[120,180)` — the arm made unreachable by the double `h<120`; must fall to `m,m,m` — randomized × S1 × L1/L2/L3 | [x] |
| 7 | `hsl_to_rgb` | H5 `h∈[180,240)` × S1 × L1/L2/L3 — randomized (X2 wrap) | [x] |
| 8 | `hsl_to_rgb` | H6 `h∈[240,300)` × S1 × L1/L2/L3 — randomized (X2 wrap) | [x] |
| 9 | `hsl_to_rgb` | H7 `h∈[300,360)` × S1 × L1/L2/L3 — randomized (X2 wrap) | [x] |
| 10 | `hsl_to_rgb` | H8 `h>=360` × S1 × L1/L2/L3 — randomized big hues | [x] |
| 11 | `hsl_to_rgb` | S0 `s = +0.0` × every H arm × randomized `l` (hue must be ignored) | [x] |
| 12 | `hsl_to_rgb` | S0 `s = -0.0` × every H arm × randomized `l` | [x] |
| 13 | `hsl_to_rgb` | S2 `s > 1` (up to `1e30`) × randomized H × randomized L | [x] |
| 14 | `hsl_to_rgb` | S3 `s < 0` × randomized H × randomized L | [x] |
| 15 | `hsl_to_rgb` | S4 `s` subnormal (`±1e-45`, `±1e-40`) × randomized H × L | [x] |
| 16 | `hsl_to_rgb` | S5 `s = ±INF` × randomized H × L (incl. L2 → `0*INF` = NaN) | [x] |
| 17 | `hsl_to_rgb` | S6 `s` NaN (quiet + signalling, both signs) × randomized H × L | [x] |
| 18 | `hsl_to_rgb` | L4 `l ∈ {0.0, 1.0}` (`c = 0`) × every H arm × randomized S1 | [x] |
| 19 | `hsl_to_rgb` | L5 `l` outside `[0,1]` (`-5..-0.001`, `1.001..5`) × randomized H × S1 | [x] |
| 20 | `hsl_to_rgb` | L6 `l = ±INF` × randomized H × S1 | [x] |
| 21 | `hsl_to_rgb` | L7 `l` NaN (both signs, several payloads) × randomized H × S1 | [x] |
| 22 | `hsl_to_rgb` | X4 `h` an exact multiple of 120 (`0, 120, 240, 360, 720, -120`) × S1 × L1/L2/L3 | [x] |
| 23 | `hsl_to_rgb` | X1/X2/X3 sawtooth sweep: `h` stepped finely across `-720 .. +1080` × S1 × L2 | [x] |
| 24 | `hsl_to_rgb` | X5 `h = ±INF` × S1 × randomized L (`x` = NaN, `c`/`m` finite) | [x] |
| 25 | `hsl_to_rgb` | X6/H9 `h` NaN (several payloads/signs) × S1 × randomized L | [x] |
| 26 | `hsl_to_rgb` | H boundary values exactly on and one ULP either side of `0,60,120,180,240,300,360` × S1 × L2 | [x] |
| 27 | `hsl_to_rgb` | Fully unconstrained fuzz: all three inputs drawn from *random 32-bit patterns* (any NaN/Inf/subnormal/huge mix) — 200 000 cases | [x] |
| 28 | `hsl_to_rgb` | Fully unconstrained fuzz restricted to *NaN-heavy* patterns (each component independently a random NaN payload/sign or a normal float) — 100 000 cases, checks NaN payload+sign propagation | [x] |
| 29 | `hsl_to_rgb` | A2 `dest == src` full aliasing × randomized valid inputs | [x] |
| 30 | `hsl_to_rgb` | A3 partial overlap `dest == src+1` and `dest == src-1` × randomized inputs | [x] |
| 31 | `hsl_to_rgb` | A4 `dest`/`src` at 4-byte-aligned offsets inside a larger shared buffer × randomized inputs | [x] |
| 32 | `hsl_to_rgb` | Repeat-call / state-freedom check: the same input called twice must give the same bits, and interleaving different rows must not change results (the C keeps no state) | [x] |

## Binary / driver

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED ...)` and no
`add_executable`; `translation/Cargo.toml` declares `crate-type = ["cdylib"]`
and has no `[[bin]]`. **The project builds no binary executable**, so the
"compare C and Rust stdout" gate is vacuously satisfied.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section and no optional
dependencies, so the complete set of feature combinations is:

| combo | command |
|-------|---------|
| default (empty) | `cargo test --release` |
| `--no-default-features` (identical to the above) | `cargo test --release --no-default-features` |
| `--all-features` (identical to the above) | `cargo test --release --all-features` |

All three are run by `run_all.sh`; there is no `#[cfg(feature = ...)]` in
`src/lib.rs`, so they compile the same code.
