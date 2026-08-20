# ERRORS.md — Error / rejection surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/lib.c`. The complete set of control-flow
decisions in the C source is:

```
grep -nE "return|assert|NULL|if |else|for " c_src/src/lib.c
  9:    for (i = 0; i < size; i++)      <- guard on `size`  (rejects size <= 0)
 11:    if (sum > 0.0f) {               <- guard on `sum`   (rejects 0 / -0 / NaN)
 13:        for (i = 0; i < size; i++)  <- guard on `size`
 15:    } else if (dest != src) {       <- guard on aliasing (rejects dest == src)
```

`normalize` returns `void`. There is **no** `return` statement, **no** error
code, **no** error enum, **no** `assert`, **no** null check, **no** min/max
constant and **no** explicit range check anywhere in the library. Therefore the
"error surface" consists of (a) the three implicit rejection guards above,
(b) the degenerate/undefined-behaviour inputs a C caller can still pass across
the FFI boundary. Every distinct rejection/degenerate branch is one row.

"expected C result" is what the C `.so` actually does (ground truth), not what
would be "reasonable".

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `normalize` | `size == 0`, `dest != src` | loop 1 skipped → `sum == 0.0f` → `sum > 0.0f` false → `dest != src` true → `memset(dest, 0, 0)` → **no byte written**, returns | `err_01_size_zero_disjoint` |
| 2  | `normalize` | `size == 0`, `dest == src` | both guards fail → **no byte written**, returns | `err_02_size_zero_inplace` |
| 3  | `normalize` | `size < 0` (e.g. `-1`, `-7`), `dest == src` | loops skipped (`0 < size` false), `dest != src` false → **no byte written**, returns normally (no crash) | `err_03_negative_size_inplace` |
| 4  | `normalize` | `size == INT_MIN`, `dest == src` | same as row 3 — **no byte written**, returns normally | `err_04_int_min_size_inplace` |
| 5  | `normalize` | `size < 0`, `dest != src` | `memset(dest, 0, (size_t)size * 4)`; `int → size_t` sign-extends, so the length is `0xFFFF_FFFF_FFFF_FFFC` for `size == -1` → runaway store → **fatal `SIGSEGV`** | `err_05_negative_size_disjoint_crashes_identically` (sub-process, signal compared) |
| 6  | `normalize` | `size == INT_MIN`, `dest != src` | length `0xFFFF_FFFE_0000_0000` → **fatal `SIGSEGV`** | `err_06_int_min_size_disjoint_crashes_identically` (sub-process) |
| 7  | `normalize` | `dest == NULL`, `src == NULL`, `size == 0` | `dest != src` false → nothing dereferenced → **returns normally** | `err_07_both_null_size_zero` |
| 8  | `normalize` | `dest == NULL`, `src != NULL`, `size == 0` | `dest != src` true → `memset(NULL, 0, 0)` → glibc `memset` with `n == 0` touches nothing → **returns normally** | `err_08_null_dest_size_zero` |
| 9  | `normalize` | `dest != NULL`, `src == NULL`, `size == 0` | loop skipped, `memset(dest, 0, 0)` → **returns normally**, `dest` untouched | `err_09_null_src_size_zero` |
| 10 | `normalize` | `dest == NULL`, `src == NULL`, `size < 0` | loops skipped, `dest != src` false → **returns normally** (no deref) | `err_10_both_null_negative_size` |
| 11 | `normalize` | `src == NULL`, `size > 0` | `src[0]` dereferenced → **fatal `SIGSEGV`** | `err_11_null_src_positive_size_crashes_identically` (sub-process) |
| 12 | `normalize` | `dest == NULL`, `size > 0`, non-zero `src` (`sum > 0`) | `dest[0]` written → **fatal `SIGSEGV`** | `err_12_null_dest_positive_size_crashes_identically` (sub-process) |
| 13 | `normalize` | `sum == 0.0f` because every `src[i]` is `±0.0f`, `dest != src` | `0.0f > 0.0f` false → `dest` fully zero-filled (`+0.0f` bit pattern `0x0000_0000`, so a `-0.0f` input is *not* copied through) | `err_13_all_zero_input_zero_fills` |
| 14 | `normalize` | `sum == 0.0f`, `dest == src` | `dest != src` false → **buffer left byte-identical** (e.g. `-0.0f` stays `0x8000_0000`) | `err_14_all_zero_input_inplace_untouched` |
| 15 | `normalize` | `sum` underflows to `0.0f` although `src` is non-zero (e.g. all elements `1e-25f`; `1e-25f*1e-25f == 0.0f`), `dest != src` | `sum > 0.0f` false → `dest` zero-filled (**not** normalised) | `err_15_underflow_to_zero_zero_fills` |
| 16 | `normalize` | `sum` underflows to `0.0f`, `dest == src` | **buffer untouched**, tiny values preserved | `err_16_underflow_to_zero_inplace_untouched` |
| 17 | `normalize` | any `src[i]` is `NaN` (any payload/sign) → `sum` is `NaN`, `dest != src` | `NaN > 0.0f` false → `dest` **zero-filled**; the `NaN` is never propagated | `err_17_nan_input_zero_fills` |
| 18 | `normalize` | any `src[i]` is `NaN`, `dest == src` | `dest != src` false → **buffer untouched**, original `NaN` payload preserved | `err_18_nan_input_inplace_untouched` |
| 19 | `normalize` | `sum` overflows to `+inf` (e.g. `src[i] == 3e38f`) | `inf > 0.0f` **true** → `sum = 1.0f/sqrtf(inf) = 0.0f` → every `dest[i] = src[i] * 0.0f` = `±0.0f` (sign of `src[i]`) | `err_19_sum_overflow_to_inf` |
| 20 | `normalize` | `src` contains `±inf` (so `sum == +inf`) | `dest[i] = inf * 0.0f` → **`NaN`** (x86 default QNaN) at the `inf` slots, `±0.0f` elsewhere; bit patterns must match exactly | `err_20_inf_element_produces_nan` |
| 21 | `normalize` | `size == 1` and `src[0]` is a denormal whose square is a denormal (`1e-20f`) | `sum > 0` true → `sum = 1/sqrtf(1e-40f)` → `dest[0] == 1.0f` or the exact C rounding | `err_21_denormal_square_still_normalises` |
| 22 | `normalize` | `dest != src` but the `memset` range overruns `src` (`dest = src - 1`, `sum == 0`) | zero-fills `size` floats starting at `dest`, clobbering `src[0..size-1]` — C performs the write anyway | `err_22_zero_fill_overruns_into_src` |
| 23 | `normalize` | out-of-range *enum* value across FFI | **N/A — the public API declares no enum, no flag and no mode parameter**; the only non-pointer parameter is `int size`, whose entire `INT_MIN..=INT_MAX` range is covered by rows 1–6 and by the `size` sweep of `CONFIGS.md` | `err_23_int_size_full_range_boundaries` (sweeps `INT_MIN`, `INT_MIN+1`, `-2`, `-1`, `0`, `1`, `2`, and `INT_MAX`-adjacent values that are safe to execute) |

| 24 | `normalize` | `size == INT_MAX` (one step past every representable valid length), `dest == src` | loop 1 reads past the buffer → **fatal `SIGSEGV`** | `err_24_int_max_size_reads_out_of_bounds_identically` (sub-process) |

## Row status (Phase C gate)

All 24 rows have a passing differential test, in both the `dev` and the
`release` profile, for the only feature combination that exists
(`--no-default-features`):

```
row  1 [x]  row  2 [x]  row  3 [x]  row  4 [x]  row  5 [x]  row  6 [x]
row  7 [x]  row  8 [x]  row  9 [x]  row 10 [x]  row 11 [x]  row 12 [x]
row 13 [x]  row 14 [x]  row 15 [x]  row 16 [x]  row 17 [x]  row 18 [x]
row 19 [x]  row 20 [x]  row 21 [x]  row 22 [x]  row 23 [x]  row 24 [x]
```

## Where the tests live

* `tests/error_paths.rs` — rows 1, 2, 3, 4, 7, 8, 9, 10, 13–22, 23 (in-process).
* `tests/crash_paths.rs` — rows 5, 6, 11, 12, 24: the scenario is executed in a
  child process against each `.so` in turn and the termination status
  (`signal`, `code`) of the two children must be **equal**; a page-aligned
  region with a `PROT_NONE` guard makes the fault address deterministic instead
  of heap-layout dependent. Measured result: both die with **signal 11
  (SIGSEGV)** for all five rows.

### Why `[profile.dev] debug-assertions = false` is required

rustc's debug-assertion runtime checks (`CheckNull`, `CheckAlignment`, integer
overflow checks) inject a *non-unwinding panic* → `abort()` (SIGABRT, signal 6)
where the C library faults (SIGSEGV, signal 11) or silently wraps. Measured on
row 11 before the fix:

```
case null_src_positive: C terminated as signal 11, Rust as signal 6
  thread '<unnamed>' panicked at src/lib.rs:42: null pointer dereference occurred
  thread caused non-unwinding panic. aborting.
```

`gcc` emits no such checks, so they are a behavioural divergence on exactly the
undefined-behaviour inputs a C caller can still pass. They are therefore
disabled for this crate (see `Cargo.toml`), which makes the `dev` and `release`
`.so`s agree with the C `.so` on every row above.
`crash_paths.rs::err_00_so_is_built_without_ub_runtime_checks` guards the
setting so it cannot silently regress.

## Boundary checklist required by Phase C

| boundary | covered by row(s) |
|----------|-------------------|
| null pointers | 7, 8, 9, 10, 11, 12 |
| zero length | 1, 2, 7, 8, 9 |
| oversized / negative length | 3, 4, 5, 6, 23, 24 |
| one step past a valid range | 3 (`-1` = one past `0`), 23 (`INT_MIN`, `INT_MIN+1`), 24 (`INT_MAX`) |
| out-of-range enum across FFI | 23 (no enum exists — justified, `int` range swept instead) |
