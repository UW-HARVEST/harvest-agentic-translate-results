# Phase A.3 — Configuration-surface table

## Axes derived mechanically from the C source

`c_src/include/lib.h` declares the complete public API — a single entry point:

```c
float ldexp_q2(float y, int exp_q2);
```

There is no context/handle struct, no option setter, no mode flag, no
`#ifdef`, no `switch`, and no `if` statement in `c_src/src/lib.c`. There is also
no "convenience wrapper vs. low-level" split: `ldexp_q2` *is* the lowest-level
entry point, and it is the only one. So the configuration surface is entirely
the cross-product of the **input shapes the arithmetic branches on**:

Axis 1 — the clamp `e = min(exp_q2, 120)` (`30 * 4`), which determines the
**iteration count** of the `do`/`while`:
* `exp_q2 <= 0` → 1 iteration, `e = exp_q2` (unclamped, negative or zero)
* `1 <= exp_q2 <= 120` → 1 iteration, `e = exp_q2` for `< 120`, `e = 120` at 120
* `exp_q2 > 120` → `ceil(exp_q2 / 120)` iterations, last one with the residual

Axis 2 — the table index `e & 3`, selecting which of the four `g_expfrac`
entries is used: residues `0, 1, 2, 3`. For negative `e` this is the
two's-complement `&`, so it still lands in `0..=3` but pairs a *negative*
quarter-step with a table entry chosen from the low bits.

Axis 3 — the integer factor `1 << 30 >> (e >> 2)`, with the shift count masked
to 5 bits by the hardware:
* `e >= 0` → count `e >> 2 ∈ 0..=30` → factor `2^30 .. 2^0` (never 0)
* `e < 0` → count `(e >> 2) & 31` → factor aliases with period 128 in `e`;
  count `31` (i.e. `e ∈ [-4,-1] mod 128`) makes the factor **0**

Axis 4 — the `float y` shape (the value-dependent axis; `f32` multiplication is
neither associative nor exact, so magnitude class matters):
* normal positive / normal negative
* `+0.0` / `-0.0`
* subnormal (positive and negative)
* `FLT_MIN`, `FLT_MAX`, `FLT_EPSILON`
* `+inf` / `-inf`
* `NaN` (default quiet, negative-sign, and non-default payload)
* fully random `f32` bit patterns (all classes mixed)

Axis 5 — the multiplication ORDER, fixed by the C source and verified in the
disassembly (`mulss %xmm1,%xmm0` = `frac * (float)shifted` first, then
`mulss` with `y`). Not a free axis, but every row asserts it implicitly since a
reassociated Rust version diverges in the low mantissa bit.

Axis 6 — feature combinations. `translation/Cargo.toml` declares **no
`[features]` section**, so the only configuration is the default (empty) feature
set; `--no-default-features` is equivalent. Recorded in row 25.

Axis 7 — binary/driver executable. Neither `c_src/CMakeLists.txt` (it only
does `add_library(... SHARED src/lib.c)`) nor `translation/Cargo.toml`
(`crate-type = ["cdylib"]`, no `[[bin]]`, no `src/main.rs`) builds an
executable. The "compare stdout of the C and Rust binaries" step is therefore
**N/A**; recorded in row 26.

## Configuration rows

Every row is exercised through **both** `.so` files via `libloading`, with many
randomized inputs per row (fixed-seed SplitMix64 PRNG), and compared on raw
`f32` bit patterns (`to_bits()`), so `-0.0` vs `+0.0` and differing `NaN`
payloads count as divergences.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `ldexp_q2` | `exp_q2 = 0` (1 iter, `e&3=0`, factor `2^30`) × random normal `y` (both signs) | [x] |
| 2 | `ldexp_q2` | `exp_q2 ∈ 1..=3` (1 iter, `e&3∈{1,2,3}`, factor `2^30`) × random normal `y` | [x] |
| 3 | `ldexp_q2` | `exp_q2 ∈ 4..=119`, all four `e&3` residues × all shift counts `1..=29` × random normal `y` (exhaustive over `exp_q2`, randomized over `y`) | [x] |
| 4 | `ldexp_q2` | `exp_q2 = 120` exactly at the `30*4` clamp (1 iter, factor `2^0 = 1`) × random `y` | [x] |
| 5 | `ldexp_q2` | `exp_q2 ∈ 121..=240` → 2 iterations, residual `1..=120` × random `y` | [x] |
| 6 | `ldexp_q2` | `exp_q2 ∈ 241..=1200` → 3..10 iterations (multi-chunk composition) × random `y` | [x] |
| 7 | `ldexp_q2` | `exp_q2` large positive (`10_000..=200_000`, hundreds–thousands of iterations) × random `y` → saturation to `±inf` | [x] |
| 8 | `ldexp_q2` | `exp_q2 = -1..=-4` → integer factor **0** × all `y` classes (incl. `inf`, `NaN`) | [x] |
| 9 | `ldexp_q2` | `exp_q2 = -5..=-128` → negative-`e` masked shift, all 4 residues, factors `2^0..2^28` × random `y` (exhaustive over `exp_q2`) | [x] |
| 10 | `ldexp_q2` | `exp_q2 = -129..=-512` → period-128 aliasing of `(e>>2)&31` × random `y` (exhaustive over `exp_q2`) | [x] |
| 11 | `ldexp_q2` | `exp_q2` large negative (`-200_000..=-10_000`) → still exactly 1 iteration, aliased factor × random `y` | [x] |
| 12 | `ldexp_q2` | `exp_q2` fully random over the whole `i32` range, **excluding** the huge-positive slow region, × fully random `f32` bit patterns (10 000 pairs) | [x] |
| 13 | `ldexp_q2` | `exp_q2` random over `i32::MIN..=0` × fully random `f32` bit patterns (10 000 pairs) | [x] |
| 14 | `ldexp_q2` | `y = +0.0` and `y = -0.0` × every `exp_q2` class (sign-of-zero preservation) | [x] |
| 15 | `ldexp_q2` | `y = +inf` / `-inf` × every `exp_q2` class (incl. the factor-0 rows ⇒ `inf*0`) | [x] |
| 16 | `ldexp_q2` | `y = NaN` (default quiet `0x7fc00000`, negative `0xffc00000`, custom payloads `0x7fc0dead`, `0x7f800001` signalling) × every `exp_q2` class | [x] |
| 17 | `ldexp_q2` | `y` subnormal (`0x00000001`, `0x007fffff`, both signs) × positive and negative `exp_q2` (gradual underflow / renormalisation) | [x] |
| 18 | `ldexp_q2` | `y = FLT_MAX` (`3.4028235e38`, both signs) × positive `exp_q2` (overflow to `inf`) and negative `exp_q2` | [x] |
| 19 | `ldexp_q2` | `y = FLT_MIN` (`1.1754944e-38`), `FLT_EPSILON`, `1.0`, `-1.0`, `2.0` × sweep of `exp_q2 ∈ -600..=600` (exhaustive cross-product) | [x] |
| 20 | `ldexp_q2` | `exp_q2 = i32::MIN`, `i32::MIN+1..+3`, `i32::MIN+4` × all `y` classes (extreme-negative boundary, `exp_q2 -= e` → 0) | [x] |
| 21 | `ldexp_q2` | `exp_q2 = i32::MAX`, `i32::MAX-1`, `i32::MAX-119`, `i32::MAX-120` × a small set of `y` values (the ~18 M-iteration path; kept to a few cases for runtime) | [x] |
| 22 | `ldexp_q2` | `exp_q2` at powers-of-two and ±1 neighbourhoods (`±2^k`, `±2^k ± 1` for `k = 0..=30`) × random `y` — shift-count and clamp boundary walk | [x] |
| 23 | `ldexp_q2` | round-trip composition: `ldexp_q2(ldexp_q2(y, a), b)` vs the same on the C side, for random `(y, a, b)` — exercises the API the way a consumer chains it (state carried in the returned value, the library's only state) | [x] |
| 24 | `ldexp_q2` | exhaustive over `exp_q2 ∈ -1024..=1024` × 8 fixed representative `y` values (dense boundary coverage of every residue/iteration-count transition in that band) | [x] |
| 25 | `ldexp_q2` | **feature combinations**: `Cargo.toml` has no `[features]`; verified under both the default feature set and `--no-default-features` (identical, empty) | [x] |
| 26 | (driver binary) | **N/A** — neither project builds an executable (`add_library(SHARED)` only; `crate-type = ["cdylib"]`, no `[[bin]]`/`src/main.rs`). Nothing to compare on stdout. | [x] |

## Adequacy check (mutation testing of the suite itself)

Passing tests only mean something if they can fail. Each mutation below was
applied to `src/lib.rs`, the `cdylib` rebuilt, and the full suite re-run against
it. "Killed" = the suite detected the divergence from the C `.so`.

| mutation | result |
|----------|--------|
| reassociate to `y * frac * shifted` (changes the `mulss` order) | **killed** (18 tests) |
| `while exp_q2 > 0` instead of the `do`/`while` | **killed** (17 tests) |
| clamp `31 * 4` (124) instead of `30 * 4` (120) | **killed** (11 tests) |
| `e / 4` (truncating division) instead of `e >> 2` | **killed** (15 tests) |
| `g_expfrac[1]` changed by one genuine ulp (`0x305744fd`→`0x305744fe`) | **killed** (16 tests) |
| shift mask `& 63` instead of `& 31` | **killed** against the debug artifact (`attempt to shift right with overflow` → SIGABRT; detected via the test exit code, which is what `scripts/verify_all.sh` checks) |
| `(e as u32) >> 2` (logical) instead of `e >> 2` (arithmetic) | survived — **provably equivalent**: `& 31` keeps only bits 2..6 of `e`, and arithmetic vs. logical shift differ only in the high bits (checked over `e ∈ [-3000, 3000] ∪ {i32::MIN, i32::MIN+1, i32::MAX, -128, -129}`: 0 mismatches) |
| `e.rem_euclid(4)` instead of `e & 3` | survived — **provably equivalent** for all `i32` (both yield the non-negative residue) |
| clamp test `>=` instead of `>` | survived — **provably equivalent** (both branches return the same value at `exp_q2 == 120`) |
| `f64` intermediate instead of `f32` | survived — **provably equivalent**: `frac * shifted` is exact in *both* `f32` and `f64` (24-bit mantissa × an exact power of two, result always normal), so both spellings perform exactly one rounding of the same exact product |
| `7.83145814e-10f` → `7.8314582e-10f` | survived — **not a mutation**: both decimals round to the identical `f32` `0x305744fd` |

Every non-equivalent mutant is killed; every survivor is shown to be a
semantically identical spelling rather than a coverage gap.
