# ERRORS.md — Phase C error-surface table

Derived mechanically. The full set of control-flow / rejection constructs in
`c_src/src/lib.c` is (grep for `return|assert|NULL|errno|RETURN_ERROR|-1|goto|exit(|abort|if (|?|<|>|enum|#define|#if`):

```
src/lib.c:7:    for (i = 0; i < count; i++) {
src/lib.c:8:        if (src[0] < src[1]) {
src/lib.c:15:  0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
src/lib.c:25:  0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
```

That is the whole surface. Consequences for the error table:

* `tfm` returns `void` — there is **no** error code, no sentinel, no
  out-parameter status. There are zero `return` statements, zero `RETURN_ERROR`
  macros, zero error enums, zero `assert`s, zero `NULL` checks, zero `errno`
  writes, and zero `#ifdef`s in the library.
* Therefore the "same error/rejection" that must be matched is, in every row
  below, an **observable behavioural rejection**: how many elements get written
  (0 vs `count`), and which sentinel float value is produced. Those are the only
  observable channels this API has.
* There are also no enum parameters at all, so the "out-of-range enum value
  across FFI" class collapses to "out-of-range `int` value for `count`", which
  is covered by rows 1–4 below.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| 1 | `tfm` | `count == 0` — loop guard `i < count` false on first iteration | no-op: **zero** bytes written to `dest`; `src` never dereferenced (so a NULL/dangling `src` is legal here) | `err_row1_count_zero` | [x] |
| 2 | `tfm` | `count < 0` (e.g. `-1`) — loop guard false, signed compare | no-op: **zero** bytes written; `src` never dereferenced | `err_row2_count_negative` | [x] |
| 3 | `tfm` | `count == INT_MIN` (most-negative `int`, one step past the negative range) | no-op: zero bytes written | `err_row3_count_int_min` | [x] |
| 4 | `tfm` | `count == 1` with `dest`/`src` sized *exactly* to the minimum (3 in / 2 out) — one step inside the valid range; verifies the C writes exactly 2 floats and reads exactly 3, never 4/6 | exactly `dest[0..2]` written, `dest[2..]` untouched; only `src[0..3]` read | `err_row4_exact_minimum_extent` | [x] |
| 5 | `tfm` | NULL `dest` and NULL `src` together with `count <= 0` (the only NULL that is *not* UB, because the pointers are never dereferenced) | no-op, no crash | `err_row5_null_pointers_count_zero` | [x] |
| 6 | `tfm` | `src[0] < src[1]` is **false because the comparison is unordered** — `src[0]` and/or `src[1]` is NaN. `comiss`/`jbe` takes the "unordered ⇒ not-less" path, so the *else* arm runs | else arm: `dest[0] = dxy`, `dest[1] = dx2 - lambda` with `dy2 = src[0]`, `dx2 = src[1]`; result is a NaN with the exact payload/sign the SSE operand order dictates | `err_row6_nan_compare_takes_else_arm` | [x] |
| 7 | `tfm` | `src[0] == src[1]` — `<` is false, so the else arm runs (boundary of the branch predicate, incl. `+0.0` vs `-0.0` which compare equal) | else arm taken, **not** the then arm | `err_row7_equal_operands_take_else_arm` | [x] |
| 8 | `tfm` | `sqd < 0` — the explicit range check `((0) > (sqd)) ? (0) : (sqd)` rejects a negative discriminant and substitutes the sentinel `0.0f` before `sqrtf`, so `sqrtf` is never called with a negative argument and `errno`/`EDOM` is never raised | `sqrtf(0.0f) == 0.0f`; `lambda == 0.5f*(dy2+dx2)` | `err_row8_negative_sqd_clamped_to_zero` | [x] |
| 9 | `tfm` | `sqd` is NaN (e.g. `dxy = inf`, `dx2 = dy2 = inf` ⇒ `inf - inf`) — the same range check is *unordered*, so the NaN is **not** clamped and reaches `sqrtf`, which quiets it | `sqrtf(NaN)` ⇒ quiet NaN propagates into `lambda` and into the stored result | `err_row9_nan_sqd_not_clamped` | [x] |
| 10 | `tfm` | `sqd == +inf` (overflow of the discriminant, e.g. huge finite `dxy`) — passes the range check, `sqrtf(+inf) = +inf` | `lambda = +inf`, `dx2 - inf = -inf` | `err_row10_inf_sqd` | [x] |
| 11 | `tfm` | oversized `count` relative to intent but consistent buffers — `count` large (e.g. 4096) with correctly sized buffers; verifies no early exit / no off-by-one over a long run | all `2*count` outputs written and identical | `err_row11_large_count` | [x] |
| 12 | `tfm` | signalling NaN in `src` (bit pattern `0x7FA0_0000` / `0xFFA0_0000`) — a value with no "valid variant"; SSE quiets it rather than trapping | quieted NaN in output, exact bits must match | `err_row12_signalling_nan` | [x] |
| 13 | `tfm` | subnormal / `±0.0` / `±FLT_MIN` / `±FLT_MAX` inputs (extremes of the representable range, where the discriminant underflows or overflows) | bit-identical output incl. sign of zero | `err_row13_subnormal_and_extremes` | [x] |
| 14 | `tfm` | `dest == src` (fully aliased buffers) — C has no aliasing guard; `dest` advances 2 while `src` advances 3, so later iterations read bytes an earlier iteration overwrote | whatever the in-place clobbering produces, byte-identical | `err_row14_aliased_dest_src` | [x] |

Rows 1–5 and 11 are the generic C-API boundaries (NULL, zero length, oversized
length, one-past-range `int`). Rows 6–10 are the two genuine in-source
rejections (`if (src[0] < src[1])` and the `0 > sqd` clamp), one row per
distinct outcome of each. Rows 12–14 are FFI-boundary values with no valid
"variant".
