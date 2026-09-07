# ERRORS.md — Phase C error / rejection surface table

Mechanical grep of the whole C source for rejection mechanisms:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|<[[:space:]]*0|>[[:space:]]*0|if' c_src/src/lib.c
18:    v = (((v) > (0)) ? (v) : (0));
23:    if (sum > 0.0f) {
```

Findings:

* `gaussian_kernel` returns `void`. There is **no error return value, no error
  enum, no sentinel, no `errno` use, no `assert`, no null check, and no range
  check** anywhere in `c_src/src/lib.c`.
* The only two conditional constructs in the entire library are the value clamp
  on line 18 and the `sum > 0.0f` guard on line 23. Neither rejects input; both
  are ordinary data-dependent branches, so they are *also* covered as valid-path
  rows in `CONFIGS.md`.
* Consequently the "error surface" of this library consists entirely of
  *implicit* rejections: conditions under which the C function silently does
  nothing, silently skips work, or produces non-finite / clamped data. Each such
  implicit rejection is one row below, with the exact observable C result that
  the Rust must reproduce.
* Passing a null / invalid `dest` with a loop that executes is undefined
  behaviour in C and is therefore **not** a testable row (both implementations
  would crash); the only null case that is well-defined is the one where the
  loop body provably never executes (row 1), and that *is* tested.

There are no enums in the API, so "out-of-range enum value" degenerates to
"out-of-range `int` value for `size`", which is covered by rows 1, 6 and 7.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `gaussian_kernel` | `size <= -2` (e.g. `-2`, `-3`, `-7`, `INT_MIN`) so `hsize = size/2 <= -1` and `-hsize > hsize` | main loop body executes **0** times; `sum` stays `0.0f`; guard `sum > 0.0f` false; **nothing at all is written**, `dest` untouched (safe even for `dest == NULL`) | [x] |
| 2 | `gaussian_kernel` | `size <= -2` **and** `dest == NULL` | no dereference happens (row 1), function returns cleanly without writing | [x] |
| 2b | `gaussian_kernel` | `size == -1` — C truncating division gives `hsize = 0`, so `-hsize == 0 <= hsize` and the loop **does** run once | writes `dest[0]` despite the negative size; `sum > 0` but normalisation loop `r < -1` runs 0 times, leaving `dest[0]` un-normalised. `dest == NULL` here is UB and is *not* tested | [x] |
| 3 | `gaussian_kernel` | `radius == 0.0f` → `rs = 1.6f/0 = +inf`; at `r == 0`, `x = 0*inf = NaN` | `x*x = NaN`, `expf(NaN)=NaN`, `1/NaN=NaN`, `NaN - s2 = NaN`, clamp `NaN > 0` is **false** → stores `+0.0f`. All `r != 0` give `x = ±inf`, `1/expf(inf) = 0`, `0 - s2 < 0` → `+0.0f`. Whole buffer `+0.0f`, `sum == 0.0f`, **normalisation loop skipped** | [x] |
| 4 | `gaussian_kernel` | `radius == -0.0f` → `rs = -inf`, same NaN/inf pattern as row 3 | identical to row 3: all `+0.0f`, no normalisation | [x] |
| 5 | `gaussian_kernel` | `radius = NaN` → `rs = NaN`, every `x` is `NaN` | every `v` is `NaN`, every clamp fails → all `+0.0f`, `sum == 0.0f`, normalisation skipped | [x] |
| 6 | `gaussian_kernel` | `size == 0` → `hsize == 0`, loop runs exactly once for `r == 0` | writes **one** `float` (`dest[0] = 1 - s2 ≈ 0.996849f`) even though the caller asked for 0 elements; `sum > 0` but the normalisation loop `r < 0` runs 0 times, so `dest[0]` is left **un-normalised** | [x] |
| 7 | `gaussian_kernel` | `size` even and `> 0` (e.g. `2`, `4`, `256`) → loop writes `2*(size/2)+1 == size+1` elements | one element is written **past** `dest[size-1]` (out-of-bounds store from the caller's point of view); the trailing element is *not* normalised because the second loop stops at `size` | [x] |
| 8 | `gaussian_kernel` | `radius` so large that `rs → 0` (e.g. `radius = 1e30f`, `+inf`) | every `x ≈ 0`, every `v = 1 - s2 > 0`, `sum` large, normalisation makes every element `1/(size_written)`; no rejection | [x] |
| 9 | `gaussian_kernel` | `radius` tiny but non-zero (e.g. `1e-30f`, `f32::MIN_POSITIVE`) → `rs = +inf` or huge | `r == 0` gives `x = 0*inf = NaN` (when `rs` overflows to `inf`) → clamped to `+0.0f`; all other `r` clamp to `+0.0f`; `sum == 0`, normalisation skipped. If `rs` is merely huge (finite), `r == 0` still gives `v = 1 - s2 > 0` and normalisation runs | [x] |
| 10 | `gaussian_kernel` | `radius` negative and finite (e.g. `-2.5f`) | `rs < 0`, but `x*x` is sign-symmetric → result **identical** to `+2.5f`; no rejection | [x] |
| 11 | `gaussian_kernel` | `radius = -inf` | `rs = -0.0f`, `x = ∓0.0`, `x*x = +0.0`, `v = 1 - s2 > 0` for every `r` → uniform normalised kernel, no rejection | [x] |
| 12 | `gaussian_kernel` | all clamped to zero except centre, `sum > 0` but denormal-small | `isum = 1/sum` may be `inf`; multiplication may yield `inf`/`NaN`; both implementations must agree bit-for-bit | [x] |
| 13 | `gaussian_kernel` | `size == INT_MIN` (extreme out-of-range `int`) | `hsize = INT_MIN/2 = -1073741824`; `-hsize = 1073741824 > hsize` → loop runs 0 times, nothing written, no normalisation. NB: in Rust `(-hsize)` for `hsize = -1073741824` does **not** overflow, matching C | [x] |
| 14 | `gaussian_kernel` | `size == 1` (minimum size that produces exactly `size` writes) | `hsize == 0`, one write, `sum > 0`, normalisation over 1 element → `dest[0] == 1.0f` exactly | [x] |
