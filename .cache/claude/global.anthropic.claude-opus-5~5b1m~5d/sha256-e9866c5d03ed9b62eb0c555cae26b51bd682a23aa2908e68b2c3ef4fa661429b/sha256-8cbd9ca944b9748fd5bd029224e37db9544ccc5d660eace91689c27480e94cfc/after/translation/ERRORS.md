# ERRORS.md — Phase C error-surface table

Mechanical grep of the entire C source for every rejection / error path.

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|-1|<[[:space:]]*0|>[[:space:]]*0|MIN|MAX' c_src/src/lib.c
11:    if (sum > 0.0f) {
15:    } else if (dest != src) {
```

## Findings

`normalize` returns `void`. There is:

* **no** `return <error>` statement (no `return -1`, no `return NULL`),
* **no** error enum / status code / `errno` write,
* **no** `assert`, `abort`, or trap,
* **no** null-pointer check,
* **no** explicit range / bounds / min / max check on `size`,
* **no** validation of `src` contents.

Therefore the library has **no explicit error-return surface**. Its entire
"rejection" behaviour is expressed as *branch selection* on the two `if`s above,
plus the implementation-defined / undefined-behaviour cases that arise when a
caller passes an input the C never validates. Each of those is one row below.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `normalize` | `size == 0`, `dest != src` — the "empty input" boundary. Sum loop body never runs so `sum` stays `0.0f`; `sum > 0.0f` is false; falls to `memset(dest, 0, 0 * 4)` | no-op. `dest` buffer left completely unmodified (0-byte `memset`). Function returns normally. | `e1_size_zero_dest_ne_src` | ✅ |
| E2 | `normalize` | `size == 0`, `dest == src` (same pointer) — empty input **and** the aliasing guard both hit | no-op. Both branches skipped (`sum > 0` false, `dest != src` false). | `e2_size_zero_dest_eq_src` | ✅ |
| E3 | `normalize` | `size < 0` (e.g. `-1`, `-7`, `INT_MIN`), `dest == src`. `for (i = 0; i < size; i++)` runs 0 times, so `sum == 0.0f`; `sum > 0.0f` false; `dest != src` false | no-op, returns normally. The dangerous `memset` is *guarded away* by the pointer equality, so this negative size is completely benign. | `e3_negative_size_dest_eq_src` | ✅ |
| E4 | `normalize` | `size < 0`, `dest != src`. Reaches `memset(dest, 0, size * sizeof(float))`. `size` (int) converts to `size_t` by sign-extension, so `-1` → `0xFFFF_FFFF_FFFF_FFFF`, then `* 4` wraps mod 2^64 → `0xFFFF_FFFF_FFFF_FFFC`. | Undefined behaviour in C: an ~18-exabyte `memset` that walks off the object and faults. Observable behaviour = **process killed by `SIGSEGV` (signal 11)**. Rust must reproduce the identical wrapping length computation and therefore the identical fault. Verified differentially in forked child processes comparing the raw wait status. | `e4_negative_size_dest_ne_src_faults` (subprocess) | ✅ |
| E5 | `normalize` | `sum` evaluates to exactly `+0.0f` because every `src[i]` is `±0.0f` (`(-0.0)*(-0.0) == +0.0`), `dest != src`. `sum > 0.0f` is **false** — this is the "reject" branch for a zero-norm vector | `dest` is zero-**filled via `memset`** (all bytes 0 ⇒ every element `+0.0f`, never `-0.0f`), *not* scaled. Note the sign of `-0.0` inputs is destroyed. | `e5_zero_norm_memset` | ✅ |
| E6 | `normalize` | zero-norm vector as E5 but `dest == src` | **no-op** — the `else if (dest != src)` guard suppresses the zeroing, so `src`'s original `-0.0f` sign bits *survive in place*. Divergent from E5: same mathematical input, different result depending only on pointer identity. | `e6_zero_norm_aliased_noop` | ✅ |
| E7 | `normalize` | `src` contains a `NaN` (quiet or signalling bit pattern), so `sum` becomes `NaN`; `dest != src`. `NaN > 0.0f` is **false** (unordered) | falls into the `memset` branch ⇒ `dest` fully zeroed. The NaN is **not** propagated to `dest`. | `e7_nan_sum_memset` | ✅ |
| E8 | `normalize` | `src` contains a `NaN`, `dest == src` | no-op; the NaN stays in `src`/`dest` unchanged (both branches skipped). | `e8_nan_sum_aliased_noop` | ✅ |
| E9 | `normalize` | `sum` overflows to `+Infinity` (e.g. several elements near `FLT_MAX`), `dest != src`. `+inf > 0.0f` is **true** | takes the scale branch: `sqrtf(+inf) = +inf`, `1.0f/+inf = +0.0f`, so every `dest[i] = src[i] * +0.0f` ⇒ `+0.0`/`-0.0` (sign of `src[i]`), and `±inf * +0.0` ⇒ `NaN`. Must match bit-for-bit incl. sign of zero and NaN payload. | `e9_inf_sum_scale_to_zero` | ✅ |
| E10 | `normalize` | `src` contains `±Infinity` (so `sum == +inf` directly), `dest != src` | as E9: `dest[i] = src[i] * 0.0f`; the infinite elements become `NaN`, the finite ones become signed zeros. | `e10_infinite_element` | ✅ |
| E11 | `normalize` | `sum` underflows/denormalises: all `src[i]` are tiny (denormal or ~`FLT_MIN`) so `sum` is a subnormal `> 0.0f`, or flushes all the way to `+0.0f` | if `sum` is subnormal-but-nonzero ⇒ scale branch with a huge `1/sqrtf(sum)`, producing `±inf` / large values. If products underflow to exactly `+0.0f` ⇒ `memset` branch instead. The `> 0.0f` comparison decides; both sides must pick the same branch. | `e11_denormal_sum` | ✅ |
| E12 | `normalize` | `dest` / `src` are **null pointers** with `size > 0` | UB in C: dereferenced immediately in the sum loop ⇒ **`SIGSEGV`**. Rust does the same raw `*src.offset(i)` read ⇒ same fault. Verified differentially in forked children comparing wait status. | `e12_null_pointer_size_positive_faults` (subprocess) | ✅ |
| E13 | `normalize` | `dest` and `src` are **null pointers** with `size == 0` | benign: no dereference, `sum == 0`, `dest != src` is **false** (both null ⇒ equal) ⇒ no `memset`. Returns normally without touching memory. | `e13_null_pointers_size_zero_ok` | ✅ |
| E14 | `normalize` | `dest` is null, `src` is a valid all-zero buffer, `size == 0` | `dest != src` is **true** (null != valid), so `memset(NULL, 0, 0)` is executed with length 0. In practice returns normally without faulting. | `e14_null_dest_size_zero` | ✅ |
| E15 | `normalize` | out-of-range "enum" value across the FFI boundary. The only non-pointer parameter is `int size`; C accepts **any** `int`, so the out-of-domain values are `INT_MIN`, `INT_MAX`, `-1`. `INT_MAX` with a real buffer would read out of bounds, so the safe differential probe is `INT_MIN`/`INT_MAX` combined with `dest == src` (E3's guard) | `INT_MIN`/`-1`: loop skipped, guard suppresses `memset`, no-op. `INT_MAX` with `dest == src` would loop 2^31-1 times reading OOB ⇒ SIGSEGV; covered as a fault-parity check. | `e15_extreme_int_sizes` | ✅ |

There is no enum type in this API, so "out-of-range enum value" degenerates to
row E15 (arbitrary `int` in the `size` parameter), which is covered.

---

## Phase C results

All 15 rows have a passing differential test. Test binary:
`translation/tests/phase_c_errors.rs` (22 tests, all green).

```
running 22 tests
e1_size_zero_dest_ne_src ................. ok      (E1)
e2_size_zero_dest_eq_src ................. ok      (E2)
e3_negative_size_dest_eq_src ............. ok      (E3)
e4_negative_size_dest_ne_src_faults ...... ok      (E4)  both sides: SIGSEGV(11)
e4b_int_min_size_dest_ne_src_faults ...... ok      (E4)  both sides: SIGSEGV(11)
e5_zero_norm_memset ...................... ok      (E5)
e6_zero_norm_aliased_noop ................ ok      (E6)
e7_nan_sum_memset ........................ ok      (E7)
e8_nan_sum_aliased_noop .................. ok      (E8)
e9_inf_sum_scale_to_zero ................. ok      (E9)
e10_infinite_element ..................... ok      (E10)
e11_denormal_sum ......................... ok      (E11)
e12_null_pointer_size_positive_faults .... ok      (E12) both sides: SIGSEGV(11)
e12b_null_src_only_faults ................ ok      (E12) both sides: SIGSEGV(11)
e12c_null_dest_scale_branch_faults ....... ok      (E12) both sides: SIGSEGV(11)
e12d_null_dest_memset_branch_faults ...... ok      (E12) both sides: SIGSEGV(11)
e13_null_pointers_size_zero_ok ........... ok      (E13)
e13b_null_pointers_negative_size_ok ...... ok      (E13)
e14_null_dest_size_zero .................. ok      (E14)
e15_extreme_int_sizes .................... ok      (E15)
e15b_int_max_size_faults ................. ok      (E15) both sides: SIGSEGV(11)
e15c_size_one_past_the_buffer ............ ok      (generic one-past-range boundary)
```

The fault-parity rows compare the exact `wait(2)` status of one child process per
side, so "both died as SIGSEGV(11)" is asserted -- not merely "both failed".

## Divergence found and fixed

**`translation/Cargo.toml`, `[profile.dev]` / `[profile.test]`.** With the
default `debug-assertions = true`, Rust's `ub_checks` instrumentation rewrote the
null-pointer UB rows (E12, E12b, E12c, E12d) into a controlled `abort()`, so the
debug-profile Rust `.so` died with **SIGABRT (6)** where the C dies with
**SIGSEGV (11)**. `debug-assertions` and `overflow-checks` are now disabled for
the `dev`/`test` profiles so *every* build profile of the translation reproduces
the C's observable behaviour. (`overflow-checks` matters for the same reason: the
`i += 1` induction variable wraps silently in C but would panic in Rust.)

Re-verified: all 22 tests pass against **both** the release `.so` and the debug
`.so`, against C built at `-O0`, `-O2` and `-O3`.
