# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -nE 'return|assert|NULL|errno|-1|ERROR|if |switch|#if|malloc|free' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:8:    e = ((30 * 4) > (exp_q2) ? (exp_q2) : (30 * 4));
c_src/src/lib.c:9:    y *= g_expfrac[e & 3] * (1 << 30 >> (e >> 2));
c_src/src/lib.c:10:    } while ((exp_q2 -= e) > 0);
c_src/src/lib.c:11:    return y;
```

The only `return` is the unconditional `return y;`. There is:

* **no** error-return macro (`RETURN_ERROR`, …), **no** `return -1`, **no**
  `return NULL`, **no** error enum, **no** out-parameter status;
* **no** `assert` / `static_assert` / `abort` / `exit`;
* **no** pointer parameter anywhere in the API, hence **no** null check to make;
* **no** explicit range check on `exp_q2` — the only clamp is the `?:` at line 8,
  which *saturates* rather than rejects;
* **no** allocation, so no allocation-failure path.

So `ldexp_q2` is **total**: every `(float, int)` pair in the domain returns a
`float` and nothing is ever rejected. The rejection surface is therefore not made
of error codes but of the *implicit* boundaries the code walks into — the
saturation constant, the two's-complement index, the shift whose count leaves
the standard-defined range, and the IEEE-754 special values. Each row below is
one distinct such condition, taken from what the source actually does, and each
has a differential test in `tests/errors.rs` asserting C and Rust return the
**same bit pattern** (the sentinel this API has instead of an error code).

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `ldexp_q2` | `exp_q2 == 0` — degenerate exponent; `do/while` still runs the body once with `e == 0`, so the `while` guard `0 > 0` fails after exactly one scaling | returns `y * (g_expfrac[0] * (1<<30))`, i.e. `y * 2^-30 * 2^30`, **not** `y` unchanged; no error |
| 2 | `ldexp_q2` | `exp_q2 < 0` (any negative), e.g. `-1` — `e = exp_q2 < 0`, so `e >> 2 < 0` and `1 << 30 >> negative` is **undefined behaviour** per C11 6.5.7p3 | no trap/error: the emitted code is `sar edx, cl`, and x86 masks a 32-bit shift count to its low 5 bits, so the result is `0x40000000 >> ((e>>2) & 31)` |
| 3 | `ldexp_q2` | `exp_q2 == -1` — worst case of row 2: `(e>>2) & 31 == 31`, so the integer factor collapses to `0` | returns `y * (g_expfrac[3] * 0.0f)` = `±0.0f` for finite `y` (sign = sign of `y`), and `NaN` for `y ∈ {±inf, NaN}` |
| 4 | `ldexp_q2` | `exp_q2 == INT_MIN` — extreme of row 2, and the value for which `-exp_q2` would overflow; `e = INT_MIN`, `e & 3 == 0`, `e >> 2 == -2^29`, `(-2^29) & 31 == 0` so **no** shift happens | returns `y * (g_expfrac[0] * (1<<30))`; the subsequent `exp_q2 -= e` is `INT_MIN - INT_MIN == 0` — the one subtraction that could overflow does not — loop exits after one pass |
| 5 | `ldexp_q2` | `exp_q2 == INT_MAX` — saturation path taken the maximum number of times: `e` clamps to `120` for `⌊INT_MAX/120⌋ = 17 895 697` iterations, then a final short iteration with the remainder `7` | returns `0.0f` for finite `y` (product underflows to zero long before the loop ends); terminates, does not hang or overflow |
| 6 | `ldexp_q2` | `exp_q2 > 120` (any) — exceeds the hard-coded ceiling `30 * 4`; the ternary silently saturates `e` to `120` instead of rejecting | multiple loop iterations, each scaling by `g_expfrac[0] * (1<<30)`; the remainder `exp_q2 % 120` is applied in the last iteration |
| 7 | `ldexp_q2` | `exp_q2 == 120` exactly — the boundary value of the clamp; `120 > 120` is false so the *else* branch is taken and `e = 30*4` from the constant, not from `exp_q2` | single iteration, `e == 120`, `e & 3 == 0`, `e >> 2 == 30`, factor `= g_expfrac[0] * 1` |
| 8 | `ldexp_q2` | `exp_q2 == 121` — one step past the clamp boundary; forces exactly two iterations (`e = 120`, then `e = 1`) | two scalings; result differs from the single-iteration `exp_q2 == 120` case by the second factor `g_expfrac[1] * (1<<30)` |
| 9 | `ldexp_q2` | negative `exp_q2` whose `e & 3` is non-zero — two's-complement `&` on a negative `int`, e.g. `-1 & 3 == 3`, `-2 & 3 == 2`, `-3 & 3 == 1` | index is *always* within `0..=3`, so `g_expfrac[e & 3]` is **never** an out-of-bounds read; a "negative index" is impossible by construction |
| 10 | `ldexp_q2` | `exp_q2` a negative multiple of `-128` (so `e >> 2` is a multiple of `-32`), e.g. `-128`, `-256` — the masked shift count wraps to `0` | integer factor is the *unshifted* `0x40000000`; distinct from the `exp_q2 == -1` collapse-to-zero of row 3 |
| 11 | `ldexp_q2` | `y` is `NaN` (quiet or signalling) — no NaN check exists | NaN propagates through both `mulss`es; a signalling NaN is quieted; result is NaN with the input payload |
| 12 | `ldexp_q2` | `y` is `±inf` with an `exp_q2` whose integer factor is `0` (row 3) — `inf * 0.0f` is the IEEE-754 invalid operation | returns the default `NaN`, no error signalled to the caller |
| 13 | `ldexp_q2` | `y` is `±0.0f` — sign must survive multiplication by a possibly-`0` factor | returns `±0.0f`; sign is the XOR of the operand signs |
| 14 | `ldexp_q2` | `y` finite but the scaling would overflow `float` range (e.g. `y = FLT_MAX` with the largest possible factor, `exp_q2 = -128` / `INT_MIN`) | **UNREACHABLE, verified.** `g_expfrac[0]` is *exactly* `2^-30` (bit pattern `0x30800000`) and the largest integer factor is `0x40000000 >> 0 == 2^30`, so the largest per-iteration multiplier is exactly `1.0f`; the other three entries are `2^-30 * 2^(-k/4) < 2^-30`. `ldexp_q2` is therefore magnitude **non-increasing** and can never turn a finite `y` into `±inf`. The test asserts this invariant on both implementations instead of asserting an overflow |
| 15 | `ldexp_q2` | `y` finite but the scaling underflows (e.g. `y = FLT_MIN`, `exp_q2` large positive) | returns a subnormal or `±0.0f`, silently |
| 16 | `ldexp_q2` | `y` is a subnormal input — smallest magnitudes, where the `float` multiply loses relative precision | scaled per IEEE-754 round-to-nearest-even; no error |
| 17 | `ldexp_q2` | "out-of-range enum" analogue: `exp_q2` is an `int` parameter with **no** valid-value documentation, so *every* one of the 2^32 bit patterns is a real input, including the ones no sane caller passes (`INT_MIN`, `INT_MAX`, `-1`, `0x80000001`, `0x7FFFFFFF`) | all are accepted; behaviour is fully determined by rows 1–10; the Rust `.so` must reproduce every one bit-exactly |

Rows 1–17 each map to a `#[test]` in `tests/errors.rs`; see the checklist at the
bottom of that file's module doc for the pass status.
