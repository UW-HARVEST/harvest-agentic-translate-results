# CONFIGS.md — Phase A configuration-surface table

## How this was derived

`c_src/include/lib.h` declares exactly **one** public entry point:

```c
uint16_t float2half(float flt);
```

There is no init/context/handle type, no setter, no option struct, no flag
argument, no global mode variable, no `#ifdef`, and no `switch`/`if` anywhere in
`c_src/src/lib.c` (see the grep in `ERRORS.md`). So:

* **Runtime options / modes / flags axis: EMPTY.** Nothing can be configured.
* **Entry-point axis: a single function.** It is simultaneously the lowest-level
  and the only-level API — there is no convenience wrapper to mistake for the
  real thing. Every row below therefore drives `float2half` directly through the
  `.so` export.

The only axes that remain are the **input-shape** axes, and the C code
distinguishes them purely through its two lookup tables. Those distinctions were
extracted mechanically by run-length-encoding `m__shift[512]` and reading the
matching `m__base` entries. `j = (n >> 23) & 0x1ff` decomposes as
`j = (sign << 8) | biased_exponent`, so the table regions are exactly the
input-shape classes:

| j run | shift | base | meaning of the float input |
|-------|-------|------|-----------------------------|
| 0–102 | 24 | 0x0000 | positive, exponent ≤ 102 → underflows binary16 to +0 |
| 103…112 | 23…14 (a *different* shift for each single j) | 0x0001…0x0200 | positive, maps to a binary16 **subnormal** |
| 113–142 | 13 | 0x0400…0x7800 | positive, maps to a binary16 **normal** |
| 143–254 | 24 | 0x7c00 | positive, exponent ≥ 143 → overflows binary16 to +inf |
| 255 | 13 | 0x7c00 | positive inf / **NaN** (mantissa is shifted, so payload survives) |
| 256–358 | 24 | 0x8000 | negative counterpart of 0–102 |
| 359…368 | 23…14 (one per j) | 0x8001…0x8200 | negative subnormal counterpart |
| 369–398 | 13 | 0x8400…0xf800 | negative normal counterpart |
| 399–510 | 24 | 0xfc00 | negative overflow to −inf |
| 511 | 13 | 0xfc00 | negative inf / NaN |

Axes and their values:

* **A. sign bit** — 2 values (`j < 256`, `j >= 256`).
* **B. exponent class** — 15 per sign: 1 underflow run, 10 singleton subnormal
  exponents (each with its own shift 23…14), 1 normal run (shift 13),
  1 overflow run (shift 24), 1 inf/NaN singleton (shift 13). *(1 + 10 + 1 + 1 + 1 = 14 —
  the 15th "class" is the run-boundary exponents, covered by rows 31–34.)*
* **C. mantissa shape** — 6 values, applied to **every** row: `0`,
  `1` (lowest bit only), `0x7fffff` (all 23 bits), `0x400000` (top bit only),
  a value whose only set bits are strictly **below** `m__shift[j]` (the bits the
  shift discards — the value-dependent truncation path), and uniformly random.

Cross-product, pruned to the combinations the tables actually treat
differently: **2 signs × 14 exponent classes = 28 rows**, each row driven with
all 6 mantissa shapes plus **1000 randomized mantissas** (seeded LCG, fixed
seed `0x2545F4914F6CDD1D`, reproducible), plus 6 whole-surface sweep rows.

Every row asserts the `u16` returned by the C `.so` and the Rust `.so` are
bit-identical.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | sign=+, exponent class = underflow run (j 0–102, shift 24, base 0x0000) × all 6 mantissa shapes + 1000 random mantissas per exponent | [x] |
| 2 | `float2half` | sign=+, j=103 (shift 23, base 0x0001) × 6 mantissa shapes + 1000 random | [x] |
| 3 | `float2half` | sign=+, j=104 (shift 22, base 0x0002) × 6 shapes + 1000 random | [x] |
| 4 | `float2half` | sign=+, j=105 (shift 21, base 0x0004) × 6 shapes + 1000 random | [x] |
| 5 | `float2half` | sign=+, j=106 (shift 20, base 0x0008) × 6 shapes + 1000 random | [x] |
| 6 | `float2half` | sign=+, j=107 (shift 19, base 0x0010) × 6 shapes + 1000 random | [x] |
| 7 | `float2half` | sign=+, j=108 (shift 18, base 0x0020) × 6 shapes + 1000 random | [x] |
| 8 | `float2half` | sign=+, j=109 (shift 17, base 0x0040) × 6 shapes + 1000 random | [x] |
| 9 | `float2half` | sign=+, j=110 (shift 16, base 0x0080) × 6 shapes + 1000 random | [x] |
| 10 | `float2half` | sign=+, j=111 (shift 15, base 0x0100) × 6 shapes + 1000 random | [x] |
| 11 | `float2half` | sign=+, j=112 (shift 14, base 0x0200) × 6 shapes + 1000 random | [x] |
| 12 | `float2half` | sign=+, exponent class = normal run (j 113–142, shift 13, base 0x0400–0x7800) × 6 shapes + 1000 random per exponent | [x] |
| 13 | `float2half` | sign=+, exponent class = overflow run (j 143–254, shift 24, base 0x7c00) × 6 shapes + 1000 random per exponent | [x] |
| 14 | `float2half` | sign=+, j=255 inf/NaN (shift 13, base 0x7c00) × 6 shapes + 1000 random mantissas (covers qNaN and sNaN payloads) | [x] |
| 15 | `float2half` | sign=−, exponent class = underflow run (j 256–358, shift 24, base 0x8000) × 6 shapes + 1000 random per exponent | [x] |
| 16 | `float2half` | sign=−, j=359 (shift 23, base 0x8001) × 6 shapes + 1000 random | [x] |
| 17 | `float2half` | sign=−, j=360 (shift 22, base 0x8002) × 6 shapes + 1000 random | [x] |
| 18 | `float2half` | sign=−, j=361 (shift 21, base 0x8004) × 6 shapes + 1000 random | [x] |
| 19 | `float2half` | sign=−, j=362 (shift 20, base 0x8008) × 6 shapes + 1000 random | [x] |
| 20 | `float2half` | sign=−, j=363 (shift 19, base 0x8010) × 6 shapes + 1000 random | [x] |
| 21 | `float2half` | sign=−, j=364 (shift 18, base 0x8020) × 6 shapes + 1000 random | [x] |
| 22 | `float2half` | sign=−, j=365 (shift 17, base 0x8040) × 6 shapes + 1000 random | [x] |
| 23 | `float2half` | sign=−, j=366 (shift 16, base 0x8080) × 6 shapes + 1000 random | [x] |
| 24 | `float2half` | sign=−, j=367 (shift 15, base 0x8100) × 6 shapes + 1000 random | [x] |
| 25 | `float2half` | sign=−, j=368 (shift 14, base 0x8200) × 6 shapes + 1000 random | [x] |
| 26 | `float2half` | sign=−, exponent class = normal run (j 369–398, shift 13, base 0x8400–0xf800) × 6 shapes + 1000 random per exponent | [x] |
| 27 | `float2half` | sign=−, exponent class = overflow run (j 399–510, shift 24, base 0xfc00) × 6 shapes + 1000 random per exponent | [x] |
| 28 | `float2half` | sign=−, j=511 inf/NaN (shift 13, base 0xfc00) × 6 shapes + 1000 random mantissas (qNaN + sNaN payloads) | [x] |
| 29 | `float2half` | **all 512 j values** (full sign × exponent cross-product) × 6 mantissa shapes — no exponent left untested | [x] |
| 30 | `float2half` | **run-boundary exponents**: every j at a shift-run edge and one step either side (102/103, 112/113, 142/143, 254/255, 358/359, 368/369, 398/399, 510/511) × 6 shapes + 1000 random | [x] |
| 31 | `float2half` | **named real-world magnitudes** fed as `float` values (not bit patterns): 0, ±1, ±2, ±0.5, ±65504 (half max), ±65520, ±65536, ±6.103515625e-5 (half min normal), ±5.960464477539063e-8 (half min subnormal), ±FLT_MIN, ±FLT_MAX, ±inf, NaN | [x] |
| 32 | `float2half` | **uniformly random 32-bit patterns** reinterpreted as `float` (1,000,000 draws, seeded) — hits arbitrary exponent/mantissa combinations including all NaN/denormal classes | [x] |
| 33 | `float2half` | **random real-valued floats** across many magnitude decades (1e-45 … 1e39, both signs, seeded) — the way an actual consumer calls the API | [x] |
| 34 | `float2half` | **exhaustive: all 2^32 bit patterns** — the complete input domain, C vs Rust | [x] |

All 34 rows checked: see `tests/differential.rs` (rows 1–33) and
`tests/exhaustive.rs` (row 34).

## Feature combinations

`translation/Cargo.toml` contains no `[features]` table, so there is exactly one
feature configuration. `cargo test`, `cargo test --no-default-features`, and
`cargo test --all-features` build identical code; all three were run, and the
table above holds for each.
