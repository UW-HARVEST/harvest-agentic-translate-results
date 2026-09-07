# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/src/lib.c`. The library has **one** public
entry point and **no** runtime options, no global state, no init/teardown, no
mode flags, no `#ifdef`s:

```c
float ldexp_q2(float y, int exp_q2);
```

## Axes the C code actually branches on

| axis | where in the C | distinct values the code distinguishes |
|------|----------------|-----------------------------------------|
| A. clamp of `exp_q2` | `e = ((30*4) > exp_q2 ? exp_q2 : (30*4))` | `exp_q2 < 120` (pass-through) vs `exp_q2 >= 120` (clamped to `120`) |
| B. loop trip count | `do { … } while ((exp_q2 -= e) > 0)` | exactly 1 iteration (`exp_q2 <= 120`) vs 2 iterations (`121..=240`) vs N > 2 iterations (`> 240`) |
| C. sub-quarter phase | `g_expfrac[e & 3]` | 4 distinct table entries, selected by `e & 3` ∈ {0,1,2,3}; reachable with both non-negative and negative `e` |
| D. integer scale / shift count | `1 << 30 >> (e >> 2)` | `e >> 2` ∈ `0..=30` → scale `2^(30-(e>>2))` (non-zero); `e >> 2 == 30` → scale `1`; `e >> 2 < 0` → **negative count**, `sar %cl` masks to `& 31`, giving `0` (for counts `31`, i.e. `e>>2 ∈ -1..`) or a large scale (for `e>>2` whose low 5 bits are small, e.g. `INT_MIN >> 2`) |
| E. `y` class | the `float` multiplications | `+normal`, `-normal`, `+0.0`, `-0.0`, `+subnormal`, `-subnormal`, `+inf`, `-inf`, quiet NaN, signalling NaN, `FLT_MAX`, `FLT_MIN`, arbitrary random bit patterns |
| F. saturation of the product | `y *= …` | in-range, overflow to `±inf`, underflow to `±0.0` |

There is no byte-order, element-type, count, or format axis: both parameters are
scalars passed by value.

## Configuration table (cross-product, pruned to what the C distinguishes)

Each row is exercised with **many randomized inputs** (fixed seed `0x5eed_1dea`,
SplitMix64 PRNG) in `tests/differential.rs`, comparing the C `.so` and the Rust
`.so` bit-for-bit (`f32::to_bits`).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| C01 | `ldexp_q2` | A=pass-through, B=1 iter, C=`e&3==0`, D=scale non-zero: `exp_q2 ∈ {0,4,8,…,116}`, `y` = random finite normals | [x] |
| C02 | `ldexp_q2` | A=pass-through, B=1 iter, C=`e&3==1`: `exp_q2 ∈ {1,5,…,117}`, `y` = random finite normals | [x] |
| C03 | `ldexp_q2` | A=pass-through, B=1 iter, C=`e&3==2`: `exp_q2 ∈ {2,6,…,118}`, `y` = random finite normals | [x] |
| C04 | `ldexp_q2` | A=pass-through, B=1 iter, C=`e&3==3`: `exp_q2 ∈ {3,7,…,119}`, `y` = random finite normals | [x] |
| C05 | `ldexp_q2` | A=clamp boundary, D=`e>>2 == 30` → scale exactly `1`: `exp_q2 == 120`, `y` = random finite | [x] |
| C06 | `ldexp_q2` | A=clamp, B=exactly 2 iterations: `exp_q2 ∈ 121..=240` (all 4 residual phases `e&3` on the 2nd iteration), `y` = random finite | [x] |
| C07 | `ldexp_q2` | A=clamp, B=N>2 iterations: `exp_q2 ∈ 241..=2000` random, `y` = random finite | [x] |
| C08 | `ldexp_q2` | A=clamp, B=very large trip count: `exp_q2 ∈ {100_000, 1_000_000}`, `y` = random finite | [x] |
| C09 | `ldexp_q2` | D=**negative shift count** (UB path), C=all 4 phases: `exp_q2 ∈ -1..=-256`, `y` = random finite normals | [x] |
| C10 | `ldexp_q2` | D=negative shift with low-5-bits variety: `exp_q2` = random negative in `-2^31..0` (covers `e>>2` whose masked count spans `0..=31`), `y` = random finite | [x] |
| C11 | `ldexp_q2` | E=`y` non-finite: `y ∈ {+inf,-inf,qNaN,sNaN}` × `exp_q2 ∈ {negative, 0, small positive, 120, 121, large}` | [x] |
| C12 | `ldexp_q2` | E=`y` zero/subnormal: `y ∈ {+0.0,-0.0,±FLT_TRUE_MIN,±subnormals}` × representative `exp_q2` set | [x] |
| C13 | `ldexp_q2` | F=overflow/underflow: `y ∈ {±FLT_MAX, ±FLT_MIN, ±1e38, ±1e-38}` × `exp_q2` sweep including negatives and clamped values | [x] |
| C14 | `ldexp_q2` | Fully unconstrained fuzz: `y` = random 32-bit pattern reinterpreted as `f32` (any class, incl. NaN payloads), `exp_q2` = random `i32` restricted to `>= -2^31 .. 100_000` to bound runtime | [x] |
| C15 | `ldexp_q2` | Exhaustive `exp_q2` sweep `-4096..=4096` × a fixed set of representative `y` values (all classes) | [x] |
| C16 | `ldexp_q2` | Extremes of `exp_q2`: `INT_MIN`, `INT_MIN+1..+8`, `INT_MAX`, `INT_MAX-1..-8` × representative `y` (the `INT_MAX` rows are the maximal-trip-count case) | [x] |

## Binary / driver

`c_src/CMakeLists.txt` declares only `add_library(... SHARED ...)` — there is
**no** executable target, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout"
requirement is therefore vacuous for this project.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table. The only combinations are
the default build and `--no-default-features` (identical). Both are run by
`tests/feature_matrix.sh`.
