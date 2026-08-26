# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c` (49 lines) and
`c_src/include/lib.h` (1 line). The grep used:

```sh
grep -n "return\|assert\|if *(\|NULL\|errno\|#if\|#ifdef\|#define" c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:37:    if (x < 129) {
c_src/src/lib.c:38:        return g_pow43[16 + x];
c_src/src/lib.c:40:    if (x < 1024) {
c_src/src/lib.c:46:    return g_pow43[16 + ((x + sign) >> 6)] *
```

**Mechanical result: the C library contains no rejection path at all.**
There is no `assert`, no `RETURN_ERROR`-style macro, no `return -1`,
no `return NULL`, no error enum, no `errno`, no pointer parameter (hence no
null check), no length/size parameter (hence no length check), and no
conditional compilation. `pow43` accepts *every* `int` value and always
returns a `float`.

The rejection surface is therefore made of the code's **implicit** limits: the
two explicit range comparisons it does perform (`x < 129`, `x < 1024`) and the
input values for which the unchecked table index
`g_pow43[16 + …]` leaves the 145-entry table (undefined behaviour in C — an
out-of-bounds read). Every one of those is a row below.

Table geometry used by the rows: `static const float g_pow43[129 + 16]`, valid
indices `0 ..= 144`.

| # | function | trigger (exact invalid input / condition) | expected C result | differential test | status |
|---|----------|-------------------------------------------|-------------------|-------------------|--------|
| 1 | `pow43` | *any* `int` — no validation exists anywhere in `lib.c` (no assert / error code / errno / sentinel) | never signals an error; returns a `float` for every accepted input. No error sentinel exists to compare, so "same rejection" == "same value" | `err01_no_validation_path_exists` | [x] |
| 2 | `pow43` | explicit range check #1 `if (x < 129)` at its edges: `x == 128` (taken) vs `x == 129` (not taken) | `x=128` → direct read `g_pow43[144]` = `645.079578`; `x=129` → shift path, `mult = 16` | `err02_boundary_129` | [x] |
| 3 | `pow43` | explicit range check #2 `if (x < 1024)` at its edges: `x == 1023` (taken) vs `x == 1024` (not taken) | `x=1023` → `mult=16`, `x <<= 3` (= 8184); `x=1024` → `mult=256`, no shift | `err03_boundary_1024` | [x] |
| 4 | `pow43` | lowest input that still indexes inside the table: `x == -16` → index `0` | `g_pow43[0]` = `+0.0f` (bits `0x00000000`) | `err04_lowest_defined_input` | [x] |
| 5 | `pow43` | one step below the valid range: `x == -17` → index `-1` (read *before* the table) | **undefined behaviour** — out-of-bounds read of `.rodata` preceding the table; value is a property of the compiled image, not of the library contract (this gcc-11 `-O0` build happens to return `+0.0`) | `err05_below_table_is_ub` (out-of-process, both images) | [x] |
| 6 | `pow43` | highest input that still indexes inside the table: `x == 8223` → index `144` | `g_pow43[144] * poly * 256` = `165974.13` (bits `0x48221588`) | `err06_highest_defined_input` | [x] |
| 7 | `pow43` | one step past the valid range: `x == 8224` → index `145` (read *past* the table) | **undefined behaviour** — out-of-bounds read; in this build it reads gcc's spilled `2.f/9` literal and returns `56.59508` | `err07_above_table_is_ub` (out-of-process, both images) | [x] |
| 8 | `pow43` | `x == INT_MAX` (and `INT_MAX-1`): `2 * x` and `x + sign` overflow (signed-overflow UB), index `-33554416` | **undefined behaviour** — wild read ≈ 128 MiB below the table; this build faults (`SIGSEGV`) in *both* images | `err08_int_max_is_ub` | [x] |
| 9 | `pow43` | `x == INT_MIN` (and `INT_MIN+1`): first branch taken, index `-2147483632` | **undefined behaviour** — wild read; this build faults (`SIGSEGV`) in *both* images | `err09_int_min_is_ub` | [x] |
| 10 | `pow43` | `x == 0` — the "zero" table entry a naive validator would treat specially | `g_pow43[16]` = `+0.0f` (positive zero, bits `0x00000000`, *not* `-0.0`) | `err10_zero_input_is_positive_zero` | [x] |
| 11 | `pow43` | division by zero: `((x & ~63) + sign) == 0` in `frac` | **unreachable** — the divide is only reached for `x >= 1024`, or for `x >= 129` after `x <<= 3` (so `x >= 1032`); hence `x & ~63 >= 1024` and `sign ∈ {0, 64}` ⇒ denominator `>= 1024`. No `int` input can make it 0, so no `inf`/`NaN` can be produced in the defined domain | `err11_denominator_never_zero` | [x] |
| 12 | `pow43` | negative numerator: `sign == 64` ⇒ `(x & 63) - sign < 0` ⇒ `frac < 0` (the only path where the polynomial *reduces* the table value; a "wrong-sign" translation bug hides here) | negative `frac`, result still positive and below the table entry × `mult` | `err12_negative_frac_branch` | [x] |
| 13 | `pow43` | NULL pointer argument | **N/A** — the public API (`float pow43(int x);`) has no pointer parameter; nothing can be null | `err13_api_has_no_pointer_or_length_args` (documents/asserts the ABI shape) | [x] |
| 14 | `pow43` | zero-length / oversized-length argument | **N/A** — the API has no length, size or count parameter | `err13_api_has_no_pointer_or_length_args` | [x] |
| 15 | `pow43` | out-of-range enum value passed across FFI | **N/A** — no `enum` appears in `lib.h`; the single parameter is a plain `int`, so *every* 32-bit pattern is a legal input. Covered by: exhaustive in-domain sweep + randomized full-`i32` sampling compared out-of-process | `err15_full_i32_domain_sampling` | [x] |
| 16 | `pow43` | one step past the *documented* valid range on both ends, generic-boundary rule: `x ∈ {-17, 8224}` plus the whole near-UB band `x ∈ [8224, 8320]` and `x ∈ [-64, -17]` | **undefined behaviour** on every one of them (index `< 0` or `> 144`); each is *called in both images* and classified; the divergence is confined to exactly these inputs | `err16_ub_band_is_exactly_the_out_of_table_indices` | [x] |

## Why rows 5, 7, 8, 9 and 16 cannot assert value equality

`g_pow43` has 145 elements and the C code indexes it **without any bounds
check**, so for `x < -16` and `x > 8223` the C program performs an
out-of-bounds read: its result is whatever bytes happen to sit next to the
table inside that particular `.so` image (for this gcc 11.5 `-O0` build:
`+0.0` before the table, and the function's own spilled float literals
`2.f/9`, `4.f/3`, `1.f` after it). That value is not a property of the C
*source*, it is a property of one compiler's `.rodata` layout — a different
compiler, `-O` level or linker produces different bytes for the same C source.

Emulating gcc's constant pool in Rust would be *fabricating* behaviour, which
is explicitly worse than reporting it, so the Rust translation keeps the
faithful unchecked read (`*g_pow43.as_ptr().offset(idx)`) and the tests assert
what is actually reproducible:

1. the **domain limits are exactly where the C stops being defined** — the last
   defined inputs (`-16`, `8223`) and the first undefined ones (`-17`, `8224`)
   are located by recomputing the C index expression (`err05`, `err06`,
   `err07`, `err16`);
2. every input inside the defined domain agrees **bit-for-bit** (8240/8240
   inputs, `phase_b_row16_exhaustive_domain_sweep`);
3. undefined inputs are still *called in both libraries*, out-of-process, so a
   fault cannot hide: the tests record and compare the termination class
   (`value` vs `signal`) and assert the two implementations never disagree in a
   way that is *defined* (i.e. no in-domain input diverges, and no undefined
   input is silently treated as in-domain by one side and not the other).

## Status

All 16 rows are covered by the 15 differential tests in
`tests/error_paths.rs` (rows 13 and 14 share
`err13_api_has_no_pointer_or_length_args`, since both are N/A for the same
mechanical reason) and all of them pass — in the `dev` and `release` profiles,
for every feature combination, and against the C library built at `-O0`, `-O1`,
`-O2`, `-O3` and `-Os`. See `VERIFICATION.md` for the run log.
