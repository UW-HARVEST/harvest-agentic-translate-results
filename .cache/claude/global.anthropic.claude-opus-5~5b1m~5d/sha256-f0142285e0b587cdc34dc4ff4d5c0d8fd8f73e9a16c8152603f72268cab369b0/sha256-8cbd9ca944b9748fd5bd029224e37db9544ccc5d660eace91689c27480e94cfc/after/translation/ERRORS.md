# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` (12 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep for rejection constructs

```
$ grep -nE 'return|assert|NULL|errno|-1|if|else|switch|#if|<|>|\?|\[' \
      src/lib.c include/lib.h
src/lib.c:4:    static const float g_expfrac[4] = {9.31322575e-10f, 7.83145814e-10f,
src/lib.c:5:                                       6.58544508e-10f, 5.53767716e-10f};
src/lib.c:8:    e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));
src/lib.c:9:    y *= g_expfrac[e & 3] * (1 << 30 >> (e >> 2));
src/lib.c:10:    } while ((exp_q2 -= e) > 0);
src/lib.c:11:    return y;
```

Findings:

* `return` appears **once** (`return y;`) — the single success path. There is no
  error return, no sentinel, no `-1`, no `NULL`.
* **No** `assert`, **no** `errno` use, **no** `RETURN_ERROR`-style macro,
  **no** error enum, **no** `if`/`switch` guard, **no** `#if`.
* **No pointer parameters at all** (`float`, `int` by value), so there is no
  null-pointer rejection and no length/size parameter to bound-check.
* **No enum parameters**, so there is no invalid-enum-value class here; the
  nearest analogue is an out-of-domain `int exp_q2`, covered below.

`ldexp_q2` therefore has **no explicit error surface**: it is a total function
over `(float, int)` that always returns a `float`. Consequently the rows below
enumerate the *implicit* rejection/clamping/undefined-behaviour boundaries the C
code actually contains — the conditions where a naive translation would diverge
or would panic instead of returning a value. "expected C result" is the observed
behaviour of the compiled C `.so` (gcc, x86-64, `sar %cl`), which is the ground
truth the Rust must reproduce.

## Error / boundary surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|----------------------------------------------|-------------------|---|
| E1 | `ldexp_q2` | Implicit clamp boundary: `exp_q2 == 120` (`30*4`); ternary `(30*4) > exp_q2` is FALSE, so `e = 120` (the clamp is taken, not the pass-through) | returns a `float`; `e=120`, `exp_q2-=e` → `0`, loop exits after 1 iteration | [x] |
| E2 | `ldexp_q2` | `exp_q2 == 119` — one step below the clamp; ternary TRUE, `e = 119` (pass-through) | returns a `float`; 1 iteration, no clamp | [x] |
| E3 | `ldexp_q2` | `exp_q2 == 121` — one step past the clamp; forces a *second* loop iteration with residual `exp_q2 = 1` | returns a `float`; exactly 2 iterations | [x] |
| E4 | `ldexp_q2` | Out-of-domain negative exponent: `exp_q2 < 0`. `e = exp_q2 < 0`, so `e >> 2 < 0` and `1 << 30 >> (e >> 2)` is a **negative shift count — UB in C**. Compiled as `sar %cl` (count taken mod 32). | no error/trap; returns `y * g_expfrac[e&3] * ((1<<30) >> ((e>>2)&31))`. The count **WRAPS mod 32, it does not saturate** — verified against the built C `.so`: `ldexp_q2(1.0f, -1..=-4) == 0.0` (masked count `31` → scale `0`), but `ldexp_q2(1.0f, -8) == 9.31322575e-10`, `(1.0f, -124) == 0.5`, and `(1.0f, -128) == 1.0` (`e>>2 == -32` → masked count `0` → scale `2^30`). A translation that clamped/saturated the count, or that used a checked shift, would diverge here. | [x] |
| E5 | `ldexp_q2` | Negative index expression: `e < 0` makes `e & 3` use two's-complement low bits (`-1 & 3 == 3`, `-2 & 3 == 2`, …). A translation using a signed remainder or an unmasked index would read **out of bounds** of `g_expfrac[4]`. | no out-of-bounds access; index is always `0..=3` | [x] |
| E6 | `ldexp_q2` | `exp_q2 == INT_MIN` (`-2147483648`): extreme of the UB path; also the value for which `exp_q2 -= e` would overflow if `e != exp_q2` | `e = INT_MIN`, `e & 3 == 0`, `e >> 2 == -536870912`, masked count `0` → scale `1<<30`; `exp_q2 -= e` is exactly `0`, no signed overflow, 1 iteration | [x] |
| E7 | `ldexp_q2` | `exp_q2 == INT_MAX` (`2147483647`): maximal iteration count (`ceil(INT_MAX/120)` ≈ 17.9 M iterations), stresses the `exp_q2 -= e` accumulator for overflow | terminates and returns `0.0` (underflow to zero); no signed overflow because `e == 120 <= exp_q2` on every clamped iteration and the final iteration has `e == exp_q2` | [x] |
| E8 | `ldexp_q2` | `exp_q2 == 0` — the do/while body still runs **once** (`do`, not `while`), unlike a pre-test loop | `e = 0`, scale `1<<30`, `frac = g_expfrac[0]`. Verified against the built C `.so`: the literal `9.31322575e-10f` rounds to **exactly** `2^-30`, so `g_expfrac[0] * (1<<30)` is **exactly `1.0f`** and the observable result is the identity (`ldexp_q2(1.0f, 0) == 1.0f`, bits `0x3f800000`). The row pins the literal's rounding: a translation whose constant differed by 1 ulp would diverge. | [x] |
| E9 | `ldexp_q2` | Non-finite / degenerate `y`: `+inf`, `-inf`, `NaN` (quiet and signalling), `+0.0`, `-0.0`, subnormals — combined with a scale of `0` (from E4) produces `inf * 0 = NaN` | no trap; IEEE-754 result propagated verbatim, including NaN payload/sign and the quieting of a signalling NaN | [x] |
| E10 | `ldexp_q2` | Overflow/underflow of the `float` product itself (e.g. `y = FLT_MAX` with `exp_q2` that scales up, `y = FLT_MIN` scaled down) | no error; saturates to `±inf` / flushes to `±0.0` per IEEE-754 round-to-nearest | [x] |

All 10 rows are covered by `tests/differential.rs`
(`phase_c_*` tests) and are checked off only after passing against **both** the
C and the Rust `.so`.
