# ERRORS.md — Phase A: error / rejection surface table

## How this table was derived (mechanical grep of `c_src/src/lib.c`)

```
$ grep -nE 'return|RETURN|assert|NULL|errno|goto|-1|\?' c_src/src/lib.c
(no matches)
$ grep -nE 'if|else|for|while|switch|case' c_src/src/lib.c
6:void normalize(float *dest, const float *src, int size) {
9:    for (i = 0; i < size; i++)          <- guard #1  (i < size)
11:    if (sum > 0.0f) {                   <- guard #2  (sum > 0.0f)
13:        for (i = 0; i < size; i++)      <- guard #3  (i < size)
15:    } else if (dest != src) {           <- guard #4  (dest != src)
```

The entire library is one `void` function. It contains:

* **no** `return <error>` / `return NULL` / `return -1`
* **no** error enum, no `errno` use, no out-parameter status
* **no** `assert`
* **no** null-pointer check
* **no** explicit range check and **no** min/max constant

Therefore every "rejection" is *implicit*: the function silently declines to do
work by falling out of a loop guard or taking the other side of a branch. Those
four guards, plus the two derived quantities they feed (`size * sizeof(float)`
as the `memset` length, and the `1.0f / sqrtf(sum)` scale), are the complete
error surface. Each row below is one distinct rejection/degenerate path the C
code actually takes, plus the generic FFI boundary cases.

Literal constants that participate: `0.0f` (accumulator init), `0.0f`
(comparison threshold in guard #2), `1.0f` (reciprocal numerator), `0` (memset
fill byte), `sizeof(float)` == 4.

Verification convention: "same result" means *bit-identical* output buffers
(compared as `u32` bit patterns, so `+0.0` vs `-0.0` and NaN payloads are
distinguished), bit-identical untouched guard bytes on both sides of the
buffers, and — for the rows that abort the process — the identical termination
signal observed from a forked child.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `normalize` | `size == 0`, `dest != src`, both non-null | guard #1 skips loop → `sum == +0.0f` → guard #2 false → guard #4 true → `memset(dest, 0, 0)` → **no bytes written**, `dest` unchanged | `err_01_size_zero_disjoint` | [x] |
| 2 | `normalize` | `size == 0`, `dest == src` | guard #4 false → **nothing happens at all**, buffer unchanged | `err_02_size_zero_aliased` | [x] |
| 3 | `normalize` | `size == 0`, `dest == NULL`, `src == NULL` | `dest == src` → guard #4 false → returns normally, **no deref of NULL** | `err_03_size_zero_both_null` | [x] |
| 4 | `normalize` | `size == 0`, `dest == NULL`, `src` non-null | guard #4 true → `memset(NULL, 0, 0)` → returns normally (glibc no-op for n==0) | `err_04_size_zero_dest_null` | [x] |
| 5 | `normalize` | `size == 0`, `dest` non-null, `src == NULL` | `memset(dest, 0, 0)` → no-op, `dest` unchanged, `src` never dereferenced | `err_05_size_zero_src_null` | [x] |
| 6 | `normalize` | `size < 0` (e.g. `-1`, `-7`), `dest == src` | guard #1 skips loop, guard #4 false → **no-op**; the wrapped `memset` length is never computed | `err_06_negative_size_aliased` | [x] |
| 7 | `normalize` | `size == INT_MIN`, `dest == src` | identical no-op (extreme of row 6) | `err_07_int_min_aliased` | [x] |
| 8 | `normalize` | `size < 0`, `dest != src` | `memset` length is `(size_t)(long)size * 4`, i.e. sign-extend then wrap: `-1 → 0xFFFF_FFFF_FFFF_FFFC`. Runs off the end of the heap → **SIGSEGV** | `err_08_negative_size_disjoint_crashes` (forked child, signal parity) | [x] |
| 9 | `normalize` | `size == INT_MIN`, `dest != src` | `memset` length `0xFFFF_FFFE_0000_0000` → **SIGSEGV** | `err_09_int_min_disjoint_crashes` (forked child) | [x] |
| 10 | `normalize` | `size == INT_MAX` with a short buffer | guard #1 lets loop #1 read past the mapping → **SIGSEGV** in the accumulation loop | `err_10_int_max_reads_oob_crashes` (forked child) | [x] |
| 11 | `normalize` | `size > 0`, every `src[i] == +0.0f` | `sum == +0.0f` → guard #2 **false** (`comiss`/`jbe` takes the not-greater path) → zero-fill branch: `dest` becomes `size*4` zero bytes | `err_11_all_plus_zero` | [x] |
| 12 | `normalize` | `size > 0`, every `src[i] == -0.0f` | `(-0.0f)*(-0.0f) == +0.0f`, so `sum == +0.0f` → zero-fill; `dest` gets **`+0.0f`**, i.e. `0x00000000`, *not* `-0.0f` | `err_12_all_minus_zero` | [x] |
| 13 | `normalize` | `size > 0`, `src` all zero **and** `dest == src` | guard #2 false, guard #4 false → **buffer left completely untouched** (no write at all) | `err_13_zero_sum_aliased_no_write` | [x] |
| 14 | `normalize` | `size > 0`, all `\|src[i]\|` small enough that every square underflows to `+0.0f` (e.g. `1e-30f`) | `sum == +0.0f` → zero-fill branch, **not** the normalize branch | `err_14_underflow_to_zero_sum` | [x] |
| 15 | `normalize` | `src` contains a quiet NaN, `dest != src` | `sum` becomes NaN; `comiss` sets PF → `jbe` taken → guard #2 **false** → zero-fill branch | `err_15_quiet_nan_zero_fill` | [x] |
| 16 | `normalize` | `src` contains a quiet NaN, `dest == src` | guard #2 false, guard #4 false → **NaN bytes left in place, untouched** (payload and sign bit preserved) | `err_16_quiet_nan_aliased_untouched` | [x] |
| 17 | `normalize` | `src` contains a *signaling* NaN bit pattern (`0x7FBFFFFF`) | same as row 15 (default FP env masks the invalid-operation trap); result is the zero-fill branch, no trap | `err_17_signaling_nan` | [x] |
| 18 | `normalize` | `src` contains `-NaN` (`0xFFC00000`) and/or several distinct NaN payloads | still NaN → zero-fill branch, regardless of payload | `err_18_nan_payload_variants` | [x] |
| 19 | `normalize` | `src` contains `+INFINITY` | `(+inf)^2 == +inf` → `sum == +inf` → guard #2 **true** → `1.0f/sqrtf(+inf) == 1.0f/+inf == +0.0f`; `dest[i] = src[i] * +0.0f` → **NaN** at the inf slot, signed zero elsewhere | `err_19_plus_inf` | [x] |
| 20 | `normalize` | `src` contains `-INFINITY` | `(-inf)^2 == +inf`, identical to row 19 | `err_20_minus_inf` | [x] |
| 21 | `normalize` | `src` contains both `+INFINITY` and `-INFINITY` | `+inf + +inf == +inf` (no `inf - inf`, because the values are *squared*) → still the normalize branch, **not** NaN | `err_21_both_infs` | [x] |
| 22 | `normalize` | `src` contains `INFINITY` **and** a NaN | NaN wins the accumulation → `sum` NaN → zero-fill branch | `err_22_inf_and_nan` | [x] |
| 23 | `normalize` | `size > 0`, values so large that the accumulation overflows to `+inf` (e.g. all `1e30f`) even though no input is inf | `sum == +inf` → normalize branch → scale `+0.0f` → every `dest[i]` is `±0.0f` with `src[i]`'s sign | `err_23_sum_overflow_to_inf` | [x] |
| 24 | `normalize` | `size > 0`, `sum` lands in the **subnormal** range (e.g. `src = [1.5e-22f]`) | guard #2 true (subnormal > 0) → `sqrtf` of a subnormal → scale ≈ `6.7e21`; result is **not** exactly `±1.0f` because of the precision lost in the subnormal square | `err_24_subnormal_sum` | [x] |
| 25 | `normalize` | out-of-range "enum" value across the FFI boundary | **N/A — the API declares no enum.** The only non-pointer parameter is `int size`; the analogous "no valid variant" inputs are the full negative half of `int` plus `INT_MAX`, covered by rows 6–10, and swept randomly | `err_25_random_int_sweep` | [x] |
| 26 | `normalize` | `dest` and `src` partially overlap (`dest = src + k`, `0 < k < size`), `sum > 0` | pointers differ, so guard #4 is irrelevant; loop #2 writes `dest[i]` and *later* reads `src[i+k]`, which loop #2 has already clobbered → output is **order-dependent** and must match C's strictly-ascending order | `err_26_overlap_forward` | [x] |
| 27 | `normalize` | `dest` and `src` partially overlap (`dest = src - k`), `sum > 0` | same, but the clobbered slots are behind the read cursor, so the output equals the disjoint result | `err_27_overlap_backward` | [x] |
| 28 | `normalize` | `dest` and `src` partially overlap and `sum <= 0` (all zeros / NaN) | pointers differ → guard #4 **true** → `memset` zeroes `size*4` bytes starting at `dest`, which stomps part of `src` | `err_28_overlap_zero_fill` | [x] |
| 29 | `normalize` | `src` non-null but `dest == NULL` with `size > 0` and `sum > 0` | loop #2 writes through NULL → **SIGSEGV** | `err_29_null_dest_positive_size_crashes` (forked child) | [x] |
| 30 | `normalize` | `src == NULL` with `size > 0` | loop #1 reads through NULL → **SIGSEGV** | `err_30_null_src_positive_size_crashes` (forked child) | [x] |

## Results

All 30 rows have a passing differential test in `tests/error_paths.rs`
(31 `#[test]`s: 30 rows + the `zz_crash_child` helper). Rows 8-10 and 29-30 are
verified for *signal parity* by re-executing the test binary in a child process
that loads only one of the two libraries; both children die with
`signal: 11 (SIGSEGV)`.

### Divergence found and fixed

Rows 29 and 30 (`NULL` pointer with `size > 0`) initially FAILED:

```
case `null_dest_positive`: C child code=None signal=Some(11)
                        but Rust child code=None signal=Some(6)
```

rustc's debug-assertion UB checks turn the raw `*src.offset(i)` load into a
checked load, so the `dev`-profile Rust `.so` printed
`panicked ... null pointer dereference occurred` and `abort()`ed (SIGABRT, 6)
where the C faults (SIGSEGV, 11). The `release` `.so` already matched. Fixed by
disabling `debug-assertions` and `overflow-checks` in every profile in
`Cargo.toml` — a C ABI replacement library must not add rejection behaviour that
the C does not have. Both profiles now produce SIGSEGV, like the C.

### Known non-observable case

Rows 8/9 pin the *fact* of the crash but cannot pin the exact `memset` length:
sign-extending vs. zero-extending `size` before the `* sizeof(float)` gives
`0xFFFF_FFFF_FFFF_FFFC` vs. `0x0000_0003_FFFF_FFFC`, and both run off the end of
the heap and raise SIGSEGV. `mutation_check.sh` mutation **M14** injects exactly
that bug and confirms it is not observable across the FFI boundary. The Rust code
is nevertheless written to match the C instruction-for-instruction here (`cltq`
then `shl $2` == `(size as usize).wrapping_mul(4)`, verified in the disassembly
of both `.so`s).
