# CONFIGS.md — Phase B: configuration / valid-input surface table

## Mechanical derivation of the axes

The library's entire public API is one declaration:

```c
float ldexp_q2(float y, int exp_q2);          /* c_src/include/lib.h */
```

There is **one** public entry point, and it is simultaneously the lowest-level
and the only one — there is no convenience wrapper layered over a lower-level
API, so "exercise the low-level entry points, not just the wrappers" is
satisfied by construction. There is **no** runtime option struct, no init/
teardown state, no global mode flag, and no `#ifdef`. The axes below are
therefore derived from the branches the C code *actually* takes.

The 12-line body:

```c
float ldexp_q2(float y, int exp_q2) {
    static const float g_expfrac[4] = {9.31322575e-10f, 7.83145814e-10f,
                                       6.58544508e-10f, 5.53767716e-10f};
    int e;
    do {
        e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));   /* [1] clamp   */
        y *= g_expfrac[e & 3] * (1 << 30 >> (e >> 2));     /* [2] [3] [4] */
    } while ((exp_q2 -= e) > 0);                           /* [5] loop    */
    return y;
}
```

Every branch/axis the code distinguishes:

| axis | source | distinct states the C treats differently |
|---|---|---|
| **A. clamp branch** `[1]` (`cmovg` in the disassembly) | `120 > exp_q2 ? exp_q2 : 120` | `exp_q2 > 120` (e = 120) / `exp_q2 <= 120` (e = exp_q2) |
| **B. sign of `e`** | consequence of A | `e > 0` / `e == 0` / `e < 0` (only reachable when `exp_q2 <= 0`) |
| **C. residue** `[2]` `e & 3` (`and $0x3`) | array index into `g_expfrac` | `0`, `1`, `2`, `3` — and separately for **negative** `e`, where the index comes from two's-complement low bits (`-1 & 3 == 3`) |
| **D. shift count** `[3]` `e >> 2` (`sar $0x2` then `sar %cl`) | `k = (e>>2) & 31` (CPU masks the count to 5 bits) | `k == 0` -> scale `2^30`; `1 <= k <= 29` -> scale `2^(30-k)`; `k == 30` -> scale `1`; `k == 31` -> scale **`0`** (annihilates `y`) |
| **E. negative-shift UB** `[3]` | `e < 0` makes the shift count negative = C UB | reproduced as the masked `sar`; `k` is **periodic in `e` with period 128** (`k == 0`, i.e. identity, iff `e % 128 == 0`) |
| **F. product order** `[4]` | `y *= frac * scale` | `(frac * (float)scale)` computed first (`mulss`), then multiplied into `y` (second `mulss`) — single-precision at every step, `FLT_EVAL_METHOD == 0` |
| **G. trip count** `[5]` | `while ((exp_q2 -= e) > 0)` | `1` trip (`exp_q2 <= 120`) / `2` / `3` / many / `ceil(INT_MAX/120) == 17895698` (max) |
| **H. `y` value shape** | IEEE-754 classes the multiplies distinguish | `+/-normal`, `+/-0.0`, `+/-inf`, quiet NaN (payload+sign), signalling NaN, `+/-` subnormal, `+/-FLT_MAX`, `+/-FLT_MIN`, arbitrary random bit patterns |

Note that **negative `e` implies exactly one trip** (if `exp_q2 <= 0` then
`e == exp_q2` and `exp_q2 - e == 0`), so axes E and G do not cross: multi-trip
runs always have `e > 0`. Rows below are the cross-product of A-H pruned to the
combinations the code actually distinguishes.

## Method for every row

Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
through their exported `ldexp_q2` symbol (never a direct Rust call), then the
two `f32` results are compared with `to_bits()` so that `+0.0` vs `-0.0` and
NaN sign/payload differences are caught. Each row is driven with **many
randomized inputs** from a fixed-seed SplitMix64 PRNG (reproducible), not a
single hand-picked value.

## The table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `ldexp_q2` | **A**:`exp_q2==0`, **B**:`e==0`, **C**:`r=0`, **D**:`k=0` scale `2^30` (identity `frac[0]*2^30 == 1.0f`), **G**:1 trip x **H**: 512 random normals | `c1_identity_random_normals` | [x] |
| C2 | `ldexp_q2` | **A**:`0<exp_q2<120`, **B**:`e>0`, **C**:`r=0` (`e in {4,8,...,116}`), **D**:`1<=k<=29`, **G**:1 trip x **H**: random normals | `c2_pos_residue0` | [x] |
| C3 | `ldexp_q2` | same but **C**:`r=1` (`e in {1,5,...,117}`) | `c3_pos_residue1` | [x] |
| C4 | `ldexp_q2` | same but **C**:`r=2` (`e in {2,6,...,118}`) | `c4_pos_residue2` | [x] |
| C5 | `ldexp_q2` | same but **C**:`r=3` (`e in {3,7,...,119}`) | `c5_pos_residue3` | [x] |
| C6 | `ldexp_q2` | **A**:`exp_q2==120` clamp boundary exactly, **D**:`k=30` -> scale `1` (the `(1<<30)>>30` corner), **G**:1 trip x **H**: random normals + specials | `c6_scale_one_k30` | [x] |
| C7 | `ldexp_q2` | **A**:`exp_q2>120` -> **G**:exactly 2 trips (`exp_q2 in 121..240`), both `e>0`, mixed residues on the 2nd trip x **H**: random normals | `c7_two_trips` | [x] |
| C8 | `ldexp_q2` | **A**:`exp_q2>120` -> **G**:exactly 3 trips (`exp_q2 in 241..360`) x **H**: random normals | `c8_three_trips` | [x] |
| C9 | `ldexp_q2` | **A**:`exp_q2>120` -> **G**:many trips (`exp_q2 in 361..12000`), covers accumulated rounding of repeated `2^-30` products incl. **flush-to-zero mid-loop** x **H**: random normals | `c9_many_trips` | [x] |
| C10 | `ldexp_q2` | **G**: maximum trip count — `exp_q2 == INT_MAX` and `INT_MAX-1`, `2147483640`, `17895698*120` (`17,895,698` iterations) x **H**: normals, inf, NaN, zeros | `c10_max_trips` | [x] |
| C11 | `ldexp_q2` | **B**:`e<0` + **E**: UB masked shift, **D**:`k==31` -> **scale `0`** (`exp_q2 in {-1,-2,-3,-4}`, all 4 residues) x **H**: random normals -> signed zero | `c11_neg_scale_zero_all_residues` | [x] |
| C12 | `ldexp_q2` | **B**:`e<0`, **D**:`k==30` -> scale `1` (`exp_q2 in {-5,-6,-7,-8}`, all 4 residues) x **H**: random normals | `c12_neg_scale_one_all_residues` | [x] |
| C13 | `ldexp_q2` | **B**:`e<0`, **D**:`k==0` -> scale `2^30`, i.e. **identity for negative exp** (`exp_q2 in {-128,-256,...}`, the period-128 lattice from axis E) x **H**: random normals | `c13_neg_identity_period128` | [x] |
| C14 | `ldexp_q2` | **B**:`e<0`, **D**: every intermediate `k in 1..29` reached via negative `e` (`exp_q2 in -124..-9`), all 4 residues x **H**: random normals | `c14_neg_all_shift_counts` | [x] |
| C15 | `ldexp_q2` | **E**: full period sweep — **exhaustive** `exp_q2 in -1000..=1000` (>15 full 128-periods, both signs, crosses `0` and the `120` clamp) x **H**: random normals | `c15_exhaustive_small_exp_random_y` | [x] |
| C16 | `ldexp_q2` | **H**:`y == +0.0` and `-0.0` (signed zero) x **A/D**: identity, scale `1`, scale `0`, scale `2^k`, multi-trip | `c16_signed_zeros_all_scales` | [x] |
| C17 | `ldexp_q2` | **H**:`y == +inf`/`-inf` x **D**: scale `> 0` (stays inf) **and** scale `== 0` (`inf*0` -> NaN, IEEE invalid) | `c17_infinities_all_scales` | [x] |
| C18 | `ldexp_q2` | **H**: quiet NaN, 8 distinct payloads incl. sign-bit-set, x **A/D**: identity, scale `0`, scale `1`, multi-trip — NaN **payload preservation** across FFI | `c18_qnan_payloads_all_scales` | [x] |
| C19 | `ldexp_q2` | **H**: signalling NaN (`0x7FA00000`, `0xFFA00000`, `0x7F800001`) x **A/D**: identity, scale `0`, multi-trip — sNaN quieting | `c19_snan_all_scales` | [x] |
| C20 | `ldexp_q2` | **H**: `+/-` subnormals (smallest `0x00000001`, largest `0x007FFFFF`, random subnormals) x **D**: scale `<1` (**gradual underflow**), scale `==1`, scale `==2^30` | `c20_subnormals_all_scales` | [x] |
| C21 | `ldexp_q2` | **H**: `+/-FLT_MAX` (`0x7F7FFFFF`), `+/-FLT_MIN` (`0x00800000`) x **D**: all scale regimes (no overflow possible since every scale `<= 1.0`) | `c21_extreme_finite_all_scales` | [x] |
| C22 | `ldexp_q2` | **H**: uniformly random **raw 32-bit patterns** reinterpreted as `f32` (hits normals, subnormals, zeros, infs, qNaN, sNaN by construction) x **A**: random `exp_q2 in -4096..4096` — the broadest property test | `c22_random_bitpatterns` | [x] |
| C23 | `ldexp_q2` | **A/G**: `exp_q2` random over the **full `int32` range**, stratified so trip counts stay bounded (all negatives + `[0,120]` + sampled large positives) x **H**: random bit patterns | `c23_full_int32_stratified` | [x] |
| C24 | `ldexp_q2` | **F**: values chosen so the intermediate `frac*scale` product is *inexact* (`r != 0`, `k` mid-range) and `y` has a full 24-bit random mantissa — catches any reassociation / double-rounding / `fma` contraction difference | `c24_rounding_sensitive` | [x] |
| C25 | `ldexp_q2` | **F/G**: multi-trip with `y` near the underflow boundary so the loop crosses normal -> subnormal -> zero *between* trips (order-of-operations sensitive) | `c25_multitrip_underflow_walk` | [x] |
| C26 | `ldexp_q2` | boundary lattice: `exp_q2 in {-1,0,1,119,120,121,239,240,241}` (clamp +/-1, trip-count transitions) x **H**: all 20 special `y` values | `c26_boundary_lattice_specials` | [x] |
| C27 | `ldexp_q2` | **INT_MIN neighbourhood**: `exp_q2 in INT_MIN..INT_MIN+8` and `INT_MAX-8..INT_MAX` — extremes of the `int32` domain, incl. the `exp_q2 -= e` no-overflow corner | `c27_int_extremes_neighbourhood` | [x] |
| C28 | `ldexp_q2` | ABI/calling-convention check: `y` passed in `xmm0` and `exp_q2` in `edi`; called repeatedly in a tight interleaved C/Rust sequence to catch any register-clobber or state leak between calls (the function is pure — no `static` mutable state) | `c28_interleaved_abi_stress` | [x] |

## Features

```
$ grep -n '\[features\]' translation/Cargo.toml   -> no match
$ grep -rn 'cfg(' translation/src/                -> no match
```

The crate declares **no `[features]`** and the source contains **no `cfg`
gates**, so the default feature set is the *only* configuration. There are no
`#ifdef`s in the C either. Phase D's "repeat B-C for every feature combination"
therefore reduces to the single default combination, which is verified
explicitly by the `scripts/verify_all.sh` loop over:

- `cargo test --release`
- `cargo test` (debug — different codegen/opt level, so a genuinely distinct
  code path for FP and shift lowering)
- `cargo test --no-default-features`
- `cargo test --all-features`

## Phase B gate

All 28 rows pass across randomized inputs. **0 rows unchecked.**
