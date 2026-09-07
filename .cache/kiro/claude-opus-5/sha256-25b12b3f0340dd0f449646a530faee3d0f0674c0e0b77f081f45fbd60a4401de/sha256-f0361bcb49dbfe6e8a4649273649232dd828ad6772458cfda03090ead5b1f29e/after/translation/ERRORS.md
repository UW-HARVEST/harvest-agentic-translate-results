# Phase A.2 — Error-surface table

## Mechanical grep of the whole C source

```
$ grep -nE 'return|assert|NULL|errno|RETURN_ERROR|-1|if |switch|#ifdef|#if |enum' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:4:    static const float g_expfrac[4] = {9.31322575e-10f, 7.83145814e-10f,
c_src/src/lib.c:5:                                       6.58544508e-10f, 5.53767716e-10f};
c_src/src/lib.c:11:    return y;
```

**The C library contains exactly ONE `return` statement (`return y;`), zero
`assert`s, zero error enums, zero error-return macros, zero null checks, zero
explicit range checks, and zero min/max rejection constants.** There are no
pointer parameters and no enum parameters, so there is no "invalid handle",
"NULL argument" or "out-of-range enum variant" path to reject. `int exp_q2`
accepts the entire `int` range as a *valid* input and `float y` accepts every
`float` bit pattern, including non-finite ones.

Consequently the error surface is not made of error codes but of **boundary and
implementation-defined/undefined-behaviour conditions that the compiled C `.so`
nevertheless resolves to a concrete result.** Each row below is one such
distinct rejection-or-degenerate condition derived from the source text, with
the result the C `.so` actually produces. The Rust must reproduce the same
concrete result bit-for-bit.

The only constant that acts as a limit in the source is `30 * 4` (= 120) in
`e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));` — the clamp that keeps
`1 << 30 >> (e >> 2)` in range for non-negative exponents. Rows 4–6 walk that
boundary.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `ldexp_q2` | `exp_q2 < 0` ⇒ `e = exp_q2 < 0` ⇒ shift count `e >> 2` is **negative**: `1 << 30 >> (e >> 2)` is **undefined behaviour** in C | Compiled to `sar %cl,%edx`; x86 masks the count to 5 bits, so the factor is `0x40000000 >> ((e >> 2) & 31)`. Rust must mask with `& 31` identically. | [x] |
| 2 | `ldexp_q2` | `exp_q2 ∈ {-1,-2,-3,-4}` ⇒ `(e >> 2) & 31 == 31` ⇒ integer factor is **exactly 0** (total cancellation of the scale) | returns `y * (g_expfrac[e&3] * 0.0f)` = `±0.0` with the sign of `y` for finite `y`; `NaN` for `y = ±inf` or `y = NaN` | [x] |
| 3 | `ldexp_q2` | `exp_q2 == 0` — the `do`/`while` body still executes once (no early-out guard); a "no-op" exponent is *not* a no-op path | `e = 0`, factor `= g_expfrac[0] * 2^30 = 1.0f` exactly (`g_expfrac[0]` is bit-exactly `2^-30`), so `y` is returned unchanged, including `-0.0` and `NaN` payloads | [x] |
| 4 | `ldexp_q2` | `exp_q2 == 120` — exactly at the `30 * 4` clamp | one iteration; `e = 120`, `exp_q2 -= e` → `0`, loop exits | [x] |
| 5 | `ldexp_q2` | `exp_q2 == 121` — one step **past** the clamp; forces a second iteration with a tiny residual exponent | iter 1: `e = 120`; iter 2: `e = 1` (`121-120`); result is the product of both factors, not `2^(121/4)` | [x] |
| 6 | `ldexp_q2` | `exp_q2 == 119` — one step **below** the clamp | single iteration, `e = 119` (unclamped) | [x] |
| 7 | `ldexp_q2` | `exp_q2 == INT_MIN` (`-2147483648`) — extreme negative; `exp_q2 -= e` computed with `e == INT_MIN` | no signed overflow (`INT_MIN - INT_MIN == 0`); single iteration; `e & 3 == 0`, `(e >> 2) & 31 == 0` ⇒ factor `1.0f` ⇒ returns `y` unchanged | [x] |
| 8 | `ldexp_q2` | `exp_q2 == INT_MIN + 1 .. INT_MIN + 3` — extreme negative, non-zero low bits | single iteration; index `e & 3 ∈ {1,2,3}` via two's-complement `&` (never out of bounds); factor `g_expfrac[e&3] * 1` | [x] |
| 9 | `ldexp_q2` | `exp_q2 == INT_MAX` (`2147483647`) — extreme positive; ~17.9 M loop iterations, `y` saturates to `±inf` (or stays `±0`/`NaN`) | overflow to `±inf` for non-zero finite `y`; `±0.0` stays `±0.0`; `NaN` stays `NaN`. No signed overflow in `exp_q2 -= e`. | [x] |
| 10 | `ldexp_q2` | `y == NaN` (quiet, and with a non-default payload / negative sign bit) | `NaN` propagated through `mulss`; payload/sign must match the C `.so` bit-for-bit | [x] |
| 11 | `ldexp_q2` | `y == ±inf` combined with a factor of `0.0` (row 2) ⇒ `inf * 0` | `NaN` (the x86 default-quiet `-nan`/`nan` produced by `mulss`), compared as raw bits | [x] |
| 12 | `ldexp_q2` | `y` subnormal / `±FLT_MIN` with a large negative scale ⇒ **underflow** to zero | gradual underflow then `±0.0`, sign preserved | [x] |
| 13 | `ldexp_q2` | `y == ±FLT_MAX` with a large positive `exp_q2` ⇒ **overflow** | `±inf` | [x] |
| 14 | `ldexp_q2` | `exp_q2` values whose masked shift wraps (`(e >> 2) & 31` period-128 aliasing for negative `e`, e.g. `-128`, `-129`, `-256`, `-508`) | factor jumps non-monotonically; each residue class must match | [x] |
| 15 | `ldexp_q2` | non-finite `y` (`±inf`) with a *huge positive* `exp_q2` (multi-iteration `inf * finite`) | stays `±inf` | [x] |
| 16 | `ldexp_q2` | "out-of-range enum value across the FFI boundary" — **N/A**: the API has no enum and no pointer parameter. The full `int` domain of `exp_q2` is covered instead by rows 1, 7, 8, 9, 14 plus the randomized full-`i32`-range sweep in Phase B. | — | [x] |

Total distinct rows requiring a differential test: **15** (row 16 is a
documented N/A). All are covered by `translation/tests/differential.rs`.
