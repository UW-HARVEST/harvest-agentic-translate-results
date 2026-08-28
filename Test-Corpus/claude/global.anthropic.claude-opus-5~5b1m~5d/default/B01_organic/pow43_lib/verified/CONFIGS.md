# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Step 1 — enumerate the runtime option/mode/flag axes

```sh
$ cat c_src/include/lib.h
float pow43(int x);
$ grep -nE '#if|#ifdef|#ifndef|switch|extern|typedef|struct|enum|static' c_src/src/lib.c
3: static const float g_pow43[129 + 16] = {
```

The public header is one line. There is **no** init/config struct, no setter, no
mode enum, no global flag, no `#ifdef`-selected behaviour, no `switch`, no byte
order or element-type parameter, and no state carried between calls — `pow43`
is a pure function of a single `int`.

Therefore the configuration surface is **entirely the shape/value of the single
`int` argument**, and the full set of public entry points is:

| entry point | level | in scope |
|-------------|-------|----------|
| `pow43(int) -> float` | this *is* the lowest-level entry point; there is no convenience wrapper above it and no internal helper below it | yes |

## Step 2 — enumerate the axes the C actually branches on

From `c_src/src/lib.c` lines 34–48:

| axis | source construct | distinct states |
|------|------------------|-----------------|
| **A1** dispatch | `if (x < 129) return g_pow43[16 + x];` | *table-only* path (no arithmetic at all) vs *computed* path |
| **A2** scale | `if (x < 1024) { mult = 16; x <<= 3; }` | `mult = 16` **and** `x` pre-multiplied by 8, vs `mult = 256` and `x` untouched |
| **A3** sign | `sign = 2 * x & 64;` — bit 6 of `2*x` is bit 5 of `x` | `sign = 0` vs `sign = 64`. On A2=`16` the deciding bit is `x & 4` (because `x` was shifted left by 3); on A2=`256` it is `x & 32` |
| **A4** `frac` polarity | `frac = (float)((x & 63) - sign) / ((x & ~63) + sign)` | `frac >= 0` (when `sign == 0`) vs `frac < 0` (when `sign == 64`, numerator becomes negative and the denominator is biased up by 64) |
| **A5** `frac == 0` exactly | same line | `poly` evaluates to exactly `1.0f` (no rounding at all) vs `poly != 1` |
| **A6** table region | `g_pow43[16 + x]` / `g_pow43[16 + ((x + sign) >> 6)]` | index `0..15` (the 16 *negative* leading entries), index `16` (the `+0.0` entry), index `17..144` (the `x^(4/3)` entries) |
| **A7** index-block position | `(x + sign) >> 6` is constant across a 64-wide block of `x` | `x & 63` at the block bottom (`0`), just below the sign flip (`31`), at the sign flip (`32`), and at the block top (`63`) |
| **A8** domain edges | unchecked subscript | `x = -16` (first in-bounds), `128`/`129` (A1 flip), `1023`/`1024` (A2 flip), `8192..8223` (index `144`, last in-bounds) |

## Step 3 — the pruned cross product

Randomization: every row is driven with **many** inputs from a fixed-seed
SplitMix64 generator (`tests/support/mod.rs`), plus the row's exact boundary
values. `N` per row is given below. Comparison is always
`c_bits == rust_bits` on the raw IEEE-754 `u32`, both obtained by `dlopen` +
`dlsym("pow43")`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `pow43` | A1=table-only, A6=`0..15`: `x = -16` exactly (lower domain edge, `T[0] = +0.0`) | [x] |
| 2  | `pow43` | A1=table-only, A6=`0..15`: `x ∈ -15..=-1`, all 15 values exhaustively (the negative leading entries) | [x] |
| 3  | `pow43` | A1=table-only, A6=`16`: `x = 0` exactly (`T[16] = +0.0`; distinguishes the two `+0.0` entries) | [x] |
| 4  | `pow43` | A1=table-only, A6=`17..144`: `x ∈ 1..=128`, all 128 values exhaustively | [x] |
| 5  | `pow43` | A1=table-only, A8: `x = 128` (last table-only) and `x = 129` (first computed) compared as a pair — the A1 flip | [x] |
| 6  | `pow43` | A1=table-only, randomized: `x ∈ -16..=128`, N = 4096 random draws | [x] |
| 7  | `pow43` | A1=computed, A2=`mult 16`, A3=`sign 0`, A4=`frac > 0`: `x ∈ 129..=1023` with `x & 4 == 0` and `x % 8 != 0`, N = 4096 | [x] |
| 8  | `pow43` | A1=computed, A2=`mult 16`, A3=`sign 0`, A5=`frac == 0` exactly: `x ∈ 129..=1023` with `x % 8 == 0` (so `(x<<3) & 63 == 0`), all 111 values exhaustively | [x] |
| 9  | `pow43` | A1=computed, A2=`mult 16`, A3=`sign 64`, A4=`frac < 0`: `x ∈ 129..=1023` with `x & 4 != 0`, N = 4096 | [x] |
| 10 | `pow43` | A1=computed, A2=`mult 16`, A8: `x = 129` (first) and `x = 1023` (last) exactly — the A2 flip lower side | [x] |
| 11 | `pow43` | A1=computed, A2=`mult 16`: `x ∈ 129..=1023`, all 895 values exhaustively | [x] |
| 12 | `pow43` | A1=computed, A2=`mult 16`, A7: every A3 transition on this path, i.e. `x & 7 ∈ {0,3,4,7}` sampled across `129..=1023`, N = 2048 | [x] |
| 13 | `pow43` | A1=computed, A2=`mult 256`, A3=`sign 0`, A4=`frac > 0`: `x ∈ 1024..=8223` with `x & 63 ∈ 1..=31`, N = 8192 | [x] |
| 14 | `pow43` | A1=computed, A2=`mult 256`, A3=`sign 0`, A5=`frac == 0` exactly: `x ∈ 1024..=8223` with `x & 63 == 0`, all 113 values exhaustively | [x] |
| 15 | `pow43` | A1=computed, A2=`mult 256`, A3=`sign 64`, A4=`frac < 0`: `x ∈ 1024..=8223` with `x & 63 ∈ 32..=63`, N = 8192 | [x] |
| 16 | `pow43` | A1=computed, A2=`mult 256`, A7=`x & 63 == 31` (largest positive numerator before the sign flip), all such `x ∈ 1024..=8223` | [x] |
| 17 | `pow43` | A1=computed, A2=`mult 256`, A7=`x & 63 == 32` (the sign flip: numerator `32-64 = -32`, denominator `+64`), all such `x ∈ 1024..=8223` | [x] |
| 18 | `pow43` | A1=computed, A2=`mult 256`, A7=`x & 63 == 63` (block top, numerator `-1`), all such `x ∈ 1024..=8223` | [x] |
| 19 | `pow43` | A1=computed, A2=`mult 256`, A8: `x = 1024` exactly — the A2 flip upper side | [x] |
| 20 | `pow43` | A1=computed, A2=`mult 256`, A6=`144`, A8: `x ∈ 8192..=8223` (the whole last in-bounds index block, including `x = 8223`, the upper domain edge) exhaustively | [x] |
| 21 | `pow43` | A1=computed, A2=`mult 256`: `x ∈ 1024..=8223`, all 7200 values exhaustively | [x] |
| 22 | `pow43` | **whole defined domain exhaustively**: every `x ∈ -16..=8223` (8240 values) — the cross product of rows 1–21 in one sweep | [x] |
| 23 | `pow43` | statelessness / no hidden globals: the defined domain replayed in a random permutation and again in reverse; each result must equal the ascending-sweep result | [x] |
| 24 | `pow43` | value-shape sanity that must hold in *both* objects: every result over the defined domain is finite (no `inf`/`NaN` — confirms `frac`'s denominator is never 0) and the sign bit pattern of `+0.0` is preserved (not `-0.0`) | [x] |
| 25 | `pow43` | idempotence across `dlopen` scope: both objects loaded once and called interleaved C/Rust/C/Rust over N = 8192 random defined-domain inputs | [x] |
| 26 | `pow43` | full `i32` randomized sweep (N = 200 000, fixed seed) with the **table-relative oracle** (see `ERRORS.md`): asserts the two objects agree on `idx`, `sign`, `frac`, `poly` and `mult` for *every* `int`, including the C's out-of-bounds region where the loaded *value* is linker-dependent | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` section, so there is exactly one code
configuration. `scripts/check_all_features.sh` still runs the whole table under
default / `--no-default-features` / `--all-features` /
`--no-default-features --all-features`.
