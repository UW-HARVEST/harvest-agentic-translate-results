# ERRORS.md — Phase C: error / rejection surface table

## Mechanical derivation

The complete C implementation is 12 lines. Grepping it for **every** rejection
mechanism a C API can use:

```
$ grep -n -E 'return|assert|NULL|errno|RETURN_ERROR|goto|exit|abort|#if' c_src/src/lib.c
11:    return y;          # <- the sole `return`, and it is the success path
```

| rejection mechanism searched | occurrences in `c_src/` |
|---|---|
| `return -1` / negative sentinel      | 0 |
| `return NULL` / null sentinel        | 0 |
| error enum / status code / `errno`   | 0 |
| `assert` / `abort` / `exit`          | 0 |
| explicit range check (`if` guard)    | 0 |
| null-pointer check                   | 0 |
| min/max validation constant          | 0 (the `30 * 4` literal is a **clamp**, not a reject) |
| `goto` error label                   | 0 |
| `#if` / `#ifdef` conditional         | 0 |

### Conclusion: the error surface is EMPTY

`float ldexp_q2(float y, int exp_q2)` is a **total function** over its entire
input domain (`float` x `int32`). It takes no pointers, no lengths, no enums,
and no buffers. It has exactly one `return` statement, and it is reachable for
every input. **There is no input for which the C code returns an error, sets a
status, or rejects.** Any test asserting "C rejects X" would be fabricating
behaviour the C does not have.

Because there is no error surface, this table instead enumerates — with the
same mechanical rigor — every **degenerate / boundary / implementation-defined
condition** that the C code's two control-flow constructs (the `?:` clamp on
line 8 and the `do/while` on line 10) can be driven into. These are the rows
gated by Phase C, and each has a differential test asserting C and Rust return
the **bit-identical** `float` (compared via `to_bits()`, so `-0.0` vs `+0.0`
and NaN payloads are distinguished — not merely "both failed somehow").

`expected C result` below is the **observed** result of the compiled C `.so`,
not a guess.

## The table

| # | function | trigger (exact invalid/boundary input or condition) | expected C result | test | [x] |
|---|----------|-----------------------------------------------------|-------------------|------|-----|
| E1 | `ldexp_q2` | `exp_q2 == 0` — clamp takes the `exp_q2` arm; shift count `0`; scale `2^30`; `frac[0]*2^30 == 1.0f` exactly | returns `y` unchanged bit-for-bit (incl. `-0.0`, `+/-inf`, qNaN payload) **except** a signalling NaN, which `mulss` quiets by setting mantissa bit 22 (`0x7FA00000 -> 0x7FE00000`, `0x7F800001 -> 0x7FC00001`) | `e1_exp_zero_is_identity` | [x] |
| E2 | `ldexp_q2` | `exp_q2 == 120` — clamp boundary hit **exactly**; `e == 120`; `exp_q2 -= e` yields `0` so exactly 1 trip | `y * 2^-30` (1 trip). `y=1.0` -> `0x30800000` | `e2_clamp_boundary_exact` | [x] |
| E3 | `ldexp_q2` | `exp_q2 == 121` — **one step past** the clamp; `e == 120`, then a 2nd trip with `e == 1` | 2 trips. `y=1.0` -> `0x305744fd` | `e3_one_past_clamp` | [x] |
| E4 | `ldexp_q2` | `exp_q2 == 119` — one step **below** the clamp; single trip, `e&3 == 3`, count `29`, scale `2` | 1 trip. `y=1.0` -> `0x309837f0` | `e4_one_below_clamp` | [x] |
| E5 | `ldexp_q2` | `exp_q2 < 0` (general) — `e` is **negative**; `e & 3` indexes `g_expfrac` with two's-complement low bits; `e >> 2` is a **negative shift count** => C **undefined behaviour** | no trap/crash; gcc x86-64 emits `sar %cl` whose count the CPU masks to 5 bits, so the shift is by `(e>>2) & 31` | `e5_negative_exp_ub_shift` | [x] |
| E6 | `ldexp_q2` | `exp_q2 in {-1,-2,-3,-4}` — `e>>2 == -1`, masked count `== 31`, so `(1<<30)>>31 == 0` and the scale **annihilates** `y` | `+0.0` (`0x00000000`) for finite `y > 0`; `-0.0` for finite `y < 0` | `e6_scale_zero_annihilates` | [x] |
| E7 | `ldexp_q2` | `exp_q2 == INT_MIN` (`-2147483648`) — extreme negative; `e&3 == 0`, `e>>2 == -536870912`, masked count `== 0`, scale `2^30`; `exp_q2 -= e` computes `INT_MIN - INT_MIN == 0` (**no signed overflow**) | returns `y` unchanged, bit-for-bit (identity, same as E1) | `e7_int_min` | [x] |
| E8 | `ldexp_q2` | `exp_q2 == INT_MIN + 1 .. INT_MIN + 4` — extreme negative, non-zero residues | matches C bit-for-bit | `e7_int_min` | [x] |
| E9 | `ldexp_q2` | `exp_q2 == INT_MAX` (`2147483647`) — maximum trip count: `ceil(2147483647/120) == 17895698` iterations of the `do/while` | terminates; `+0.0` for finite `y` (underflows after a few trips) | `e9_int_max` | [x] |
| E10 | `ldexp_q2` | `y == +INFINITY` / `-INFINITY` with a scale of `0` (E6 trigger) — the IEEE-754 **invalid operation** `inf * 0` | quiet NaN. Asserted bit-identical to C's NaN, incl. sign+payload | `e10_inf_times_zero_scale` | [x] |
| E11 | `ldexp_q2` | `y == NaN` (quiet, several distinct payloads incl. sign bit set) | NaN propagated; payload preserved bit-for-bit | `e11_nan_propagation` | [x] |
| E12 | `ldexp_q2` | `y == signalling NaN` (`0x7FA00000`, `0xFFA00000`, `0x7F800001`) — sNaN across the FFI boundary | quieted by setting mantissa bit 22, **payload and sign otherwise preserved** (`0x7FA00000 -> 0x7FE00000`, `0xFFA00000 -> 0xFFE00000`, `0x7F800001 -> 0x7FC00001`) — note this differs from the `inf*0` default indefinite of E10 | `e12_snan_quieting` | [x] |
| E13 | `ldexp_q2` | `y == +0.0` / `-0.0` — **signed-zero** sign propagation through the multiplies | zero with sign `sign(y) ^ sign(scale)`; scale >= 0 so sign preserved | `e13_signed_zero` | [x] |
| E14 | `ldexp_q2` | `y` subnormal (`0x00000001` smallest positive subnormal, `0x007FFFFF` largest) => **gradual underflow to zero**. Note the total multiplier is `frac[e&3] * 2^(30-k)`, so a *scale* of `1` (`k == 30`) is still a multiplier of `~2^-30`; **only `k == 0` is the identity** | `+/-0.0` for every `k != 0` (even the largest subnormal flushes at `2^-30`); unchanged only at `k == 0` (`exp_q2 == 0` and the negative 128-lattice) | `e14_subnormal_underflow` | [x] |
| E15 | `ldexp_q2` | `y == FLT_MAX` (`0x7F7FFFFF`) / `-FLT_MAX`, and `y == FLT_MIN`; combined with identity and annihilating scales | no overflow to inf (all scales are `<= 1.0`); bit-identical | `e15_extreme_finite` | [x] |
| E16 | `ldexp_q2` | every `exp_q2` residue class `e & 3 in {0,1,2,3}` for **negative** `e` (indices produced by two's complement, e.g. `-1 & 3 == 3`) — confirms no out-of-bounds read differs | in-bounds index `0..3`; bit-identical | `e16_negative_residue_classes` | [x] |
| E17 | `ldexp_q2` | **exhaustive** sweep of the whole "small" `exp_q2` neighbourhood `-1000 ..= 1000` crossed with 12 special `y` values — catches any off-by-one in the clamp, residue, or shift masking | bit-identical for all 2001 x 12 pairs | `e17_exhaustive_small_exp_all_special_y` | [x] |
| E18 | `ldexp_q2` | full-`int32`-range randomized `exp_q2` (stratified so trip counts stay bounded) — the "out-of-range value across the FFI boundary" class | bit-identical | `e18_full_int_range_random` | [x] |

## Boundary classes that do NOT apply (documented, not skipped)

The Phase C instructions ask for null pointers, zero/oversized lengths, and
out-of-range enum values. Mechanically, from the single declaration
`float ldexp_q2(float y, int exp_q2);`:

| generic boundary class | applicability | reasoning |
|---|---|---|
| **null pointer** args | **N/A** | The API has no pointer parameter. `grep -c '\*' c_src/include/lib.h` is 0. There is no pointer to pass as null. |
| **zero / oversized length** | **N/A** | No length, size, count, or buffer parameter exists. |
| **out-of-range enum value** | **N/A as an *invalid* value** | The API declares no `enum`. `grep -c enum c_src/` is 0. The only integer parameter is a plain `int`, for which **all 2^32 values are valid input** — there is no "no valid variant" value. This class is nonetheless covered as far as it can be: rows E5-E9, E17 and E18 push `exp_q2` across its entire `int32` domain including both extremes and the region past the internal `120` clamp, which is the closest analogue of an out-of-range value and is exactly where the C's UB shift lives. |
| **struct / union padding** | **N/A** | No aggregate types cross the boundary. |
| **return-value error sentinel** | **N/A** | Return type is `float`; every bit pattern is a legitimate result, so no value is reserved as a sentinel. |

## Phase C gate

All 18 rows have a passing differential test. **0 rows unchecked.**
