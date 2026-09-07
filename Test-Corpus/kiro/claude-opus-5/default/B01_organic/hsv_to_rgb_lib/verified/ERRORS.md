# ERRORS.md — Phase C error / rejection surface table

## Mechanical derivation

Every grep for an error-reporting construct over the whole C source
(`c_src/src/lib.c`, `c_src/include/lib.h`) returns **nothing**:

```
$ grep -n 'assert\|NULL\|errno\|ERROR\|error\|return -1\|enum\|#if\|#define\|_MIN\|_MAX' src/lib.c include/lib.h
  <no matches>

$ grep -n 'return' src/lib.c include/lib.h
  src/lib.c:16:        return;      # bare early return, void function
```

Consequences, stated explicitly so the table below is not mistaken for an
oversight:

* `hsv_to_rgb` returns `void`. **There is no error channel at all** — no return
  code, no sentinel, no out-parameter status, no `errno` write, no global flag.
* There are no `assert`s, no null checks, no range checks, no clamping, and no
  min/max constants.
* There are no enums anywhere in the API, so the "out-of-range enum value across
  FFI" class is **structurally N/A** for this library (row 15 records this).
* There is no length/count/size parameter — the arity is fixed at 3 `float`s by
  the contract — so "zero and oversized lengths" is **structurally N/A**
  (row 16). The nearest analogue is a short buffer, which is plain UB in both.

The only branches in the entire function are `if (s == 0)` (line 12) and
`switch (i)` (lines 24–55). The table therefore enumerates the *rejection-shaped*
branch, the generic C-API boundaries the task mandates, and every input value
class that steers the undefined `(int)floorf(...)` conversion — which is the
real behavioural cliff in this function.

"Expected C result" below is the *observed* behaviour of the reference build on
this platform (x86-64, glibc, `cvttss2si`), which is the ground truth the Rust
must match byte-for-byte.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `hsv_to_rgb` | `src[1] == 0.0f` exactly (the only rejection-shaped branch, line 12) | early return; `dest[0..3] = v, v, v`; `h` never divided, `switch` never reached |
| 2 | `hsv_to_rgb` | `src[1] == -0.0f` (IEEE `-0.0 == 0.0` is true, so the branch IS taken) | same as row 1: `dest = v, v, v` — NOT the chromatic path |
| 3 | `hsv_to_rgb` | `src[1]` = smallest positive subnormal (`1e-45`, one step past exact zero) | branch NOT taken; falls through to the full chromatic computation |
| 4 | `hsv_to_rgb` | `src[1] = NaN` (`s == 0` is false for NaN) | branch NOT taken; `p`, `q`, `t` all NaN; selected arm mixes NaN with `v` |
| 5 | `hsv_to_rgb` | `dest == NULL` (null out-pointer) | UB: SIGSEGV on the first store (compared as a crash-signal differential) |
| 6 | `hsv_to_rgb` | `src == NULL` (null in-pointer) | UB: SIGSEGV on the load of `src[0]` (compared as a crash-signal differential) |
| 7 | `hsv_to_rgb` | `src[0] = NaN`, `s != 0` → `floorf(NaN) = NaN`, `(int)NaN` is UB | `cvttss2si` yields `INT_MIN` (`-2147483648`) ⇒ `default:` arm ⇒ `r,g,b = v,p,q` (all NaN via `f`) |
| 8 | `hsv_to_rgb` | `src[0] = +INFINITY`, `s != 0` → `(int)+inf` is UB | `INT_MIN` ⇒ `default:` arm; `f = inf - (-2147483648.0f) = inf` |
| 9 | `hsv_to_rgb` | `src[0] = -INFINITY`, `s != 0` → `(int)-inf` is UB | `INT_MIN` ⇒ `default:` arm; `f = -inf` |
| 10 | `hsv_to_rgb` | `src[0]` so large that `h/60 >= 2^31` (e.g. `1e30`, `FLT_MAX`) — out of `int` range | `INT_MIN` ⇒ `default:` arm; `f = h/60 + 2147483648.0f` |
| 11 | `hsv_to_rgb` | `src[0]` so negative that `h/60 < -2^31` (e.g. `-1e30`, `-FLT_MAX`) | `INT_MIN` ⇒ `default:` arm |
| 12 | `hsv_to_rgb` | `src[0]/60` exactly `2147483648.0f` (`= 2^31`, first value past `INT_MAX`) | out of range ⇒ `INT_MIN` ⇒ `default:` arm |
| 13 | `hsv_to_rgb` | `src[0]/60` exactly `-2147483648.0f` (`= -2^31`, last in-range value) | IN range ⇒ `i = INT_MIN` ⇒ `default:` arm (same arm, but reached legitimately) |
| 14 | `hsv_to_rgb` | `src[0] >= 300` so `i >= 5` — no valid `case`, i.e. hue past the documented `[0,360)` range with no clamping | `default:` arm ⇒ `r,g,b = v,p,q` (no wraparound, no rejection) |
| 15 | `hsv_to_rgb` | `src[0] < 0` so `i` is negative (e.g. `-30` ⇒ `i = -1`) — one step past the low end of the valid hue range | `default:` arm ⇒ `r,g,b = v,p,q` (negative hues are NOT normalised) |
| 16 | `hsv_to_rgb` | `src[1] > 1.0f` (e.g. `2.0`, `1e30`, `+inf`) — out of the documented `[0,1]` saturation range, unchecked | no rejection; `p = v*(1-s)` goes negative / non-finite and is written out verbatim |
| 17 | `hsv_to_rgb` | `src[1] < 0.0f` (e.g. `-1.0`, `-inf`) — out of `[0,1]`, unchecked | no rejection; `p,q,t` computed with negative `s` and written out verbatim |
| 18 | `hsv_to_rgb` | `src[2]` (value) `= NaN` / `±inf` / negative / `-0.0` — out of the documented `[0,1]` range, unchecked | no rejection; `v` propagates verbatim into `p,q,t` and `dest` (incl. `-0.0` sign and `inf*0 = NaN`) |
| 19 | `hsv_to_rgb` | out-of-range **enum** value passed across the FFI boundary | **N/A** — the API declares no enum and no integer parameter; grep for `enum` in the C source returns nothing. Recorded so the class is provably covered, not skipped. |
| 20 | `hsv_to_rgb` | zero / oversized **length** argument | **N/A** — the API takes no length, count or size parameter; the arity is fixed at 3 `float`s. A short `dest`/`src` buffer is out-of-contract UB in C and in Rust alike, with no defined result to compare. Recorded for completeness. |

## Status

| # | test | status |
|---|------|--------|
| 1 | `err_01_s_exactly_zero` | [x] pass |
| 2 | `err_02_s_negative_zero` | [x] pass |
| 3 | `err_03_s_smallest_subnormal` | [x] pass |
| 4 | `err_04_s_nan` | [x] pass |
| 5 | `err_05_null_dest` (crash-signal differential, forked child) | [x] pass |
| 6 | `err_06_null_src` (crash-signal differential, forked child) | [x] pass |
| 7 | `err_07_h_nan` | [x] pass |
| 8 | `err_08_h_pos_inf` | [x] pass |
| 9 | `err_09_h_neg_inf` | [x] pass |
| 10 | `err_10_h_huge_positive` | [x] pass |
| 11 | `err_11_h_huge_negative` | [x] pass |
| 12 | `err_12_h_div60_exactly_2pow31` | [x] pass |
| 13 | `err_13_h_div60_exactly_neg_2pow31` | [x] pass |
| 14 | `err_14_i_ge_5_default_arm` | [x] pass |
| 15 | `err_15_negative_hue_negative_i` | [x] pass |
| 16 | `err_16_s_above_one` | [x] pass |
| 17 | `err_17_s_below_zero` | [x] pass |
| 18 | `err_18_v_out_of_range` | [x] pass |
| 19 | `err_19_enum_surface_absent` (asserts the class is structurally absent) | [x] pass (N/A documented) |
| 20 | `err_20_length_surface_absent` (asserts the class is structurally absent) | [x] pass (N/A documented) |
