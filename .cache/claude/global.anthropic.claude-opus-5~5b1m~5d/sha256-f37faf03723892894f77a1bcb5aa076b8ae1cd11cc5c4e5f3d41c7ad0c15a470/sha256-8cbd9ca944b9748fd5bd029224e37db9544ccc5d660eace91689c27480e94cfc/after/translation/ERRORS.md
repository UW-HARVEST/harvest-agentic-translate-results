# ERRORS.md — Error / rejection surface table

Derived mechanically by grepping `c_src/src/lib.c` for every `return`, `assert`,
`NULL` check, explicit range check, and min/max constant:

```
grep -n "return\|assert\|NULL\|INT_MAX\|INT_MIN\|isnan\|default\|sizeof" c_src/src/lib.c
```

The library has **no error enum, no `RETURN_ERROR` macro, no `errno` use, no
`assert`, and no `NULL` guards.** Every public function returns an ordinary
value. The complete rejection/clamping/sentinel surface is therefore the set of
guarded branches below (`lib.c:40-47`, `lib.c:69-71`) plus the generic FFI
boundary conditions every C API has.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX` — overflow guard, `lib.c:40` (e.g. `2147483648.0`, `1e15`, `1e300`) | returns `INT_MAX` = `2147483647` | `err_01_sdti_over_int_max` | [x] |
| 2 | `safe_double_to_int` | `d == +INFINITY` (extreme case of row 1) | returns `INT_MAX` | `err_02_sdti_pos_inf` | [x] |
| 3 | `safe_double_to_int` | `d < (double)INT_MIN` — underflow guard, `lib.c:42` (e.g. `-2147483649.0`, `-1e15`, `-1e300`) | returns `INT_MIN` = `-2147483648` | `err_03_sdti_under_int_min` | [x] |
| 4 | `safe_double_to_int` | `d == -INFINITY` (extreme case of row 3) | returns `INT_MIN` | `err_04_sdti_neg_inf` | [x] |
| 5 | `safe_double_to_int` | `isnan(d)` — NaN guard, `lib.c:44`. Reached only after both comparisons are false, which NaN makes so. Quiet NaN, signalling NaN, and negative NaN all count. | returns `0` | `err_05_sdti_nan` | [x] |
| 6 | `safe_double_to_int` | exact boundary `d == (double)INT_MAX` (`2147483647.0`) — *not* `>`, so falls to the cast | returns `2147483647` (no clamp) | `err_06_sdti_boundary_int_max` | [x] |
| 7 | `safe_double_to_int` | exact boundary `d == (double)INT_MIN` (`-2147483648.0`) — *not* `<`, so falls to the cast | returns `-2147483648` (no clamp) | `err_07_sdti_boundary_int_min` | [x] |
| 8 | `safe_double_to_int` | one step past the valid range in ULPs: `nextafter(INT_MAX, +inf)` and `nextafter(INT_MIN, -inf)` | `INT_MAX` / `INT_MIN` | `err_08_sdti_one_ulp_past` | [x] |
| 9 | `safe_double_to_int` | in-range fractional / negative-zero (`2147483646.9`, `-0.0`, `-0.5`) — truncation toward zero | `(int)d` truncated | `err_09_sdti_truncation` | [x] |
| 10 | `safe_double_to_int` | subnormal / tiny magnitudes (`f64::MIN_POSITIVE`, `5e-324`) | `0` | `err_10_sdti_subnormal` | [x] |
| 11 | `process_with_fallthrough` | `code` matches no `case` → `default:` at `lib.c:69` (any `code` outside `{0,1,2,3,4,5}`, e.g. `6`, `-1`, `INT_MAX`, `INT_MIN`) — this is the function's only rejection path | returns the sentinel `-1`, ignoring `base_value` | `err_11_pwf_default_sentinel` | [x] |
| 12 | `process_with_fallthrough` | `code == 0` → the `result = 0` case, which *discards* `base_value` (a second sentinel-like path distinct from `default`) | returns `0` | `err_12_pwf_zero_case` | [x] |
| 13 | `process_with_fallthrough` | one step past each end of the valid case range: `code == -1` and `code == 6` | both return `-1` (`default`) | `err_13_pwf_one_past_range` | [x] |
| 14 | `process_with_fallthrough` | out-of-range "enum-like" `code` values crossing the FFI boundary — a C `switch` on `int` accepts *any* `int`, so values with no valid case are real inputs: `INT_MIN`, `INT_MAX`, `-2147483648`, `0x7fffffff`, `1<<31`-wrapped | all return `-1` | `err_14_pwf_ffi_enum_out_of_range` | [x] |
| 15 | `process_with_fallthrough` | `base_value` at `INT_MAX` / near `INT_MAX` with a fall-through case → signed-overflow wrap in `result += …` | wraps two's-complement (e.g. `code=5, base=INT_MAX` → `INT_MAX+150` wrapped) | `err_15_pwf_base_value_overflow` | [x] |
| 16 | `copy_data_block` | `dest == NULL` and/or `src == NULL` → `memcpy(NULL, …)` at `lib.c:78`. **Undefined behaviour in C (segfault).** Not differentially testable without crashing the harness; asserted as "both sides are UB, therefore excluded" and documented rather than executed. | UB / SIGSEGV | *documented, not executed* | [x] |
| 17 | `copy_data_block` | `dest == src` (fully aliased self copy) and every **disjoint** `dest`/`src` offset | destination unchanged / copied verbatim, byte-identical | `err_17_cdb_self_copy`, `err_17b_cdb_aliasing_in_place` | [x] |
| 17a | `copy_data_block` | **partially** overlapping `dest`/`src` (0 < offset < 40) | genuine UB with **no stable C answer**: at `-O0` GCC emits a call to glibc `memcpy` (loads all 40 bytes, then stores → overlap-safe), at `-O2` GCC inlines a forward load/store sequence whose earlier stores clobber later loads, so the two C builds disagree with *each other*. Excluded — see note below. | *documented, not asserted; the testable (aliased + disjoint) part is covered by* `err_17b_cdb_aliasing_in_place` | [x] |
| 18 | `copy_data_block` | `src` holding non-`char`-safe bytes: an unterminated 20-byte `label` (no NUL), all-`0xFF` bytes, and a NaN/inf `value` — `memcpy` must copy all `sizeof(DataBlock)` = 40 bytes *including padding* verbatim | all 40 bytes (incl. the 4 bytes of padding after `id` and the 4 trailing pad bytes) copied identically | `err_18_cdb_raw_bytes_and_padding` | [x] |
| 19 | `handle_pointer_operations` | `value * 2` overflows `int` (`value > INT_MAX/2` or `< INT_MIN/2`, e.g. `INT_MAX`, `INT_MIN`, `0x40000000`) — signed overflow, `lib.c:83` | wraps two's-complement, then `+100` (also wrapping) | `err_19_hpo_mul_overflow` | [x] |
| 20 | `handle_pointer_operations` | `*ptr + 100` overflows after a non-overflowing double (`value == INT_MAX/2`) | wraps two's-complement | `err_20_hpo_add_overflow` | [x] |
| 21 | `overunder` | `a % 6` where `a == INT_MIN` (`lib.c:115`) — the truncated-remainder edge; C99 gives `INT_MIN % 6 == -2`, which hits `default` → `-1` | `switch_result == -1` | `err_21_ou_int_min_modulo` | [x] |
| 22 | `overunder` | `a % 6` negative for any `a < 0` → the remainder is negative, so `default` is taken for every negative `a` not divisible by 6 | `switch_result == -1` for negative non-multiples; `0` when `a % 6 == 0` | `err_22_ou_negative_modulo` | [x] |
| 23 | `overunder` | `d*d + a*a` overflows `int` (`lib.c:106`) → the sum can become **negative**, so `sqrt()` of a negative double yields NaN, which `safe_double_to_int` maps to `0` | `conv4 == 0` via the NaN branch | `err_23_ou_sqrt_of_negative` | [x] |
| 24 | `overunder` | `a * 1.5` / `b * 2.7` exceed `INT_MAX` after the widening to `double` (e.g. `a == INT_MAX`) → clamped by row 1 | `conv1 == INT_MAX` / `conv2 == INT_MAX` | `err_24_ou_conv_clamping` | [x] |
| 25 | `overunder` | `a * 1.5` / `b * 2.7` below `INT_MIN` (e.g. `a == INT_MIN`) → clamped by row 3 | `conv1 == INT_MIN` / `conv2 == INT_MIN` | `err_25_ou_conv_clamping_neg` | [x] |
| 26 | `overunder` | `total` accumulation overflows `int` across the 8 additions + 5 array additions (`lib.c:133-152`) | wraps two's-complement | `err_26_ou_total_overflow` | [x] |
| 27 | `overunder` | all four arguments at every extreme corner (`INT_MIN`/`-1`/`0`/`1`/`INT_MAX`, full 5⁴ = 625 cross-product) — combined boundary sweep | identical return value *and* identical stdout | `err_27_ou_extreme_corners` | [x] |
| 28 | *(generic)* | fixed constants `1e15` / `-1e15` hard-coded at `lib.c:136,140` must always print `2147483647` / `-2147483648` | those two lines are invariant in stdout for every input | `err_28_ou_fixed_overflow_lines` | [x] |
| 29 | *(generic)* | `strncpy(label, "Source", sizeof(label)-1)` = 19 bytes + explicit `label[19] = '\0'` (`lib.c:121-122`) — the destination must be `"Source"` followed by **13 NUL pad bytes**, and byte 19 NUL | `label` bytes == `"Source\0\0\0\0\0\0\0\0\0\0\0\0\0\0"` (20 bytes) and `%s` prints `Source` | `err_29_ou_strncpy_zero_padding` | [x] |

## Notes on rows deliberately not executed

Row 16 (`NULL` into `copy_data_block`) is genuine undefined behaviour in the C:
`memcpy` is called unconditionally with no guard, so the C `.so` segfaults. A
differential test cannot observe "the same error code" because there is none —
the process dies. The Rust translation uses `copy_nonoverlapping` on the same
raw pointers, so it is *equally* UB and equally segfaults; making the Rust
return early on NULL would be a behavioural **divergence**, not a fix. The row
is recorded, justified, and excluded from execution.

Row 17a (**partially** overlapping `dest`/`src`) is excluded for a different and
stronger reason: it was actually tested, and the *C itself* gave two different
answers depending on optimisation level. `run_all.sh` builds the C library three
ways (`-O0`, `-O2`, and the flags `CMakeLists.txt` specifies) precisely so this
kind of instability is caught rather than assumed away. Since no single C
behaviour exists for that input, no Rust behaviour can match "the C", and the
row is documented instead of asserted.

The translation *is* nonetheless hardened where a well-defined C behaviour
exists: the original Rust used `core::ptr::copy_nonoverlapping`, whose
`debug_assertions` precondition check **aborted the process** on the fully
aliased call `copy_data_block(p, p)` — a real divergence, since the C returns
normally. It now reads the 40 bytes into a temporary before storing them, which
reproduces glibc's load-then-store `memcpy` for every aliased and disjoint
input. This was found by running the suite against a *debug*-profile Rust
`.so`, not just the release one.
