# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

## How this table was derived

`c_src` was grepped mechanically for every rejection construct:

```sh
grep -nE 'return|RETURN|assert|NULL|errno|abort|exit|_MAX|_MIN|if[[:space:]]*\(' -r c_src/src c_src/include
```

The complete output is two lines:

```
c_src/src/lib.c:11:    if (sum > 0.0f) {
c_src/src/lib.c:15:    } else if (dest != src) {
```

So, factually: **`normalize` performs no validation whatsoever.** There is no
`RETURN_ERROR` macro, no `return -1`, no `return NULL`, no error enum, no
`assert`, no null check, no range check, and no min/max constant. The function
returns `void`, so it has no error channel at all.

The rejection surface is therefore made of the *implicit* rejections — the two
`if` conditions above — plus the boundary/degenerate inputs the C accepts
silently and the inputs on which the C traps. "Expected C result" below is the
observable effect (bytes written / signal raised), because there is no return
value.

Reference implementation (ground truth):

```c
void normalize(float *dest, const float *src, int size) {
    float sum = 0.0f;
    int i;
    for (i = 0; i < size; i++)
        sum += src[i] * src[i];
    if (sum > 0.0f) {
        sum = 1.0f / sqrtf(sum);
        for (i = 0; i < size; i++)
            dest[i] = src[i] * sum;
    } else if (dest != src) {
        memset(dest, 0, size * sizeof(float));
    }
}
```

Two facts drive most rows:

* `sum > 0.0f` is **false** for `sum == +0.0` and for `sum == NaN`
  (unordered compare). Those inputs fall into the `else if`.
* `size * sizeof(float)` promotes `int size` to `size_t` by the usual
  arithmetic conversions, so a negative `size` becomes a near-`2^64` byte
  count.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `normalize` | `size == 0`, `dest != src`, both non-null | both loops skipped; `sum = +0.0`; `else if` taken; `memset(dest, 0, 0)` → **nothing written**; returns | `err_e1_size0_disjoint` | [x] |
| E2 | `normalize` | `size == 0`, `dest == src` | loops skipped; `sum = +0.0`; `dest != src` false → **nothing written**; returns | `err_e2_size0_inplace` | [x] |
| E3 | `normalize` | `size == -1` (negative), `dest == src` | `i < size` false immediately; `sum = +0.0`; `dest != src` false → **nothing written**; returns normally (no trap) | `err_e3_negative_size_inplace` | [x] |
| E4 | `normalize` | `size == -1` (negative), `dest != src` | `memset(dest, 0, (size_t)(-1) * 4)` = `0xFFFF_FFFF_FFFF_FFFC` bytes → writes until unmapped page → **SIGSEGV** | `err_e4_negative_size_disjoint_crash` (subprocess) | [x] |
| E5 | `normalize` | `size == INT_MIN`, `dest == src` | negative → loops skipped, `dest == src` → **nothing written**, returns | `err_e5_int_min_inplace` | [x] |
| E6 | `normalize` | `size == INT_MIN`, `dest != src` | `memset` length `= (size_t)INT_MIN * 4 mod 2^64 = 0xFFFF_FFFE_0000_0000` → **SIGSEGV** | `err_e6_int_min_disjoint_crash` (subprocess) | [x] |
| E7 | `normalize` | `src == NULL`, `size == 0`, `dest` valid | loop never dereferences `src`; `else if` taken; `memset(dest,0,0)` → **nothing written**, no trap | `err_e7_null_src_size0` | [x] |
| E8 | `normalize` | `src == NULL`, `size > 0`, `dest` valid | `src[0]` dereferences NULL → **SIGSEGV** | `err_e8_null_src_positive_size_crash` (subprocess) | [x] |
| E9 | `normalize` | `dest == NULL` **and** `src == NULL`, `size == 0` | loops skipped; `dest != src` is false (both NULL) → **nothing written**, no trap | `err_e9_both_null_size0` | [x] |
| E10 | `normalize` | `dest == NULL` **and** `src == NULL`, `size == -8` | loops skipped; `dest == src` → `memset` **not** reached → no trap | `err_e10_both_null_negative_size` | [x] |
| E11 | `normalize` | `dest == NULL`, `src` valid with `sum > 0`, `size > 0` | first loop fine, then `dest[0] = …` writes to NULL → **SIGSEGV** | `err_e11_null_dest_sum_positive_crash` (subprocess) | [x] |
| E12 | `normalize` | `dest == NULL`, `src` valid all-zeros (`sum == 0`), `size > 0` | `else if` taken (`NULL != src`) → `memset(NULL, 0, size*4)` → **SIGSEGV** | `err_e12_null_dest_sum_zero_crash` (subprocess) | [x] |
| E13 | `normalize` | `dest == NULL`, `src == NULL`, `size > 0` | `src[0]` read from NULL → **SIGSEGV** (read fault precedes any write) | `err_e13_both_null_positive_size_crash` (subprocess) | [x] |
| E14 | `normalize` | `src` contains a NaN (quiet or signalling payload), `size > 0`, `dest != src` | `sum` becomes NaN; `NaN > 0.0f` is **false** → `else if` → `dest` fully **zero-filled** (`+0.0` ×`size`); the NaN is *not* propagated | `cfg_nan_*` / `err_e14_nan_rejected_to_zero` | [x] |
| E15 | `normalize` | `src` contains a NaN, `size > 0`, `dest == src` | `sum` NaN → not `> 0`; `dest != src` false → **nothing written**, `src` keeps its original bytes (NaN payload preserved) | `err_e15_nan_inplace_untouched` | [x] |
| E16 | `normalize` | all elements zero (`+0.0` / `-0.0` mix), `size > 0`, `dest != src` | `sum == +0.0` → not `> 0` → **zero-fill**; note `-0.0` inputs become `+0.0` outputs | `err_e16_all_zero_disjoint` | [x] |
| E17 | `normalize` | all elements zero, `size > 0`, `dest == src` | **nothing written** — a `-0.0` element stays `-0.0` (0x8000_0000), it is *not* normalised to `+0.0` | `err_e17_all_zero_inplace` | [x] |
| E18 | `normalize` | non-zero `src` whose squares all underflow to zero (e.g. `1e-30f`, `x*x == 0` in `f32`), `size > 0`, `dest != src` | `sum == +0.0` even though `src != 0` → `else if` → **zero-fill**. Silent loss of the input. | `err_e18_underflow_to_zero_disjoint` | [x] |
| E19 | `normalize` | same underflowing `src`, `dest == src` | **nothing written**; the tiny values survive unchanged | `err_e19_underflow_to_zero_inplace` | [x] |
| E20 | `normalize` | `src` magnitudes large enough that `sum` overflows to `+inf` (e.g. `1e30f` repeated) | `+inf > 0.0f` is **true** → `sum = 1.0f/sqrtf(+inf) = 1.0f/+inf = +0.0` → every `dest[i] = src[i] * 0.0f` = `±0.0` (sign of `src[i]`). Output is **not** a unit vector. | `err_e20_sum_overflow_to_inf` | [x] |
| E21 | `normalize` | `src` contains `±inf` but no NaN | `inf*inf = +inf`, `sum = +inf > 0` → scale `= +0.0` → finite elements → `±0.0`, infinite elements → `inf * 0.0 =` **NaN** (default quiet NaN, positive sign on x86-64) | `err_e21_inf_input_produces_nan` | [x] |
| E22 | `normalize` | `sum` is a positive *denormal* (e.g. a single element `= 1e-22f`, `sum ≈ 1e-44` denormal) | `sum > 0` true → `sqrtf(denormal)` is normal-ish, `1.0f/…` may be huge/`+inf` → `dest[i]` may become `±inf` or overflow. Must match bit-for-bit. | `err_e22_denormal_sum` | [x] |
| E23 | `normalize` | `size == INT_MAX` with a small real buffer | first loop reads far past the allocation → **SIGSEGV** | `err_e23_int_max_size_crash` (subprocess) | [x] |
| E24 | `normalize` | out-of-range *enum* value crossing the FFI boundary | **N/A by construction** — `c_src/include/lib.h` declares no `enum`, no `typedef`, no mode/flag parameter (`grep -E '#ifdef|#if |switch|enum|typedef|extern' -r c_src/src c_src/include` → no match). The only non-pointer parameter is a plain `int size`, whose entire out-of-range space (negative, `0`, `INT_MIN`, `INT_MAX`) is covered by rows E1–E6 and E23. | documented in `err_e24_no_enum_surface` | [x] |
| E25 | `normalize` | misaligned `float*` (e.g. `dest` at a byte offset of 1) | **Not exercised.** Undefined behaviour in *both* C and Rust; there is no defined C result to treat as ground truth, so a differential test would compare two UB executions. Deliberately excluded rather than asserted. | n/a (documented) | [x] |

### Non-rows (checked and ruled out)

* `sum == -0.0` is unreachable: `sum` starts at `+0.0` and `x*x` is either
  `+0.0`, positive, or NaN, and `+0.0 + +0.0 == +0.0`. No `-0.0` row exists.
* No error code / sentinel exists to compare, because the function is `void`.
  Every row above is asserted on the *observable* effect: the exact bytes of
  `dest` (plus untouched guard bands on either side of it) for the
  non-trapping rows, and the exact termination signal of a forked child for
  the trapping rows.

## Results

All 25 rows have a passing differential test in `tests/errors.rs`
(26 tests: 25 rows + a self-check that the trap detector is not vacuous).
Non-trapping rows compare the full scratch buffer bit-for-bit *and* assert the
documented C effect; trapping rows re-exec the test binary once per
implementation and compare the terminating signal.

```
cargo test --test errors   ->  26 passed; 0 failed   (dev and release profiles)
```

### Divergence found and fixed

**Row E8 / E11 / E12 / E13 (NULL dereference), dev profile only.**

The C `.so` dies with `SIGSEGV` (11) on a NULL dereference. The Rust `.so`
built in the dev profile died with `SIGABRT` (6) and the message
`null pointer dereference occurred`: rustc's `-Cub-checks` are on by default
whenever `debug-assertions` is on, and they insert a null/alignment assert in
front of every language-level raw dereference. `core::ptr::write_bytes`
additionally carries an *enabled* `check_language_ub` precondition assert, which
aborted for row E12 even though the C `memset` would have been reached with a
harmless length.

That is an observable difference at the ABI boundary, so the Rust was changed
(the C was not):

* the element loads/stores now go through `p.wrapping_add(i).read()` /
  `.write(v)` instead of `*p.offset(i)` — `wrapping_add` has no preconditions
  and `ptr::read`/`ptr::write` carry no enabled UB check, so the faulting access
  reaches the hardware exactly as the C compiler's does;
* the zero-fill now calls libc `memset` directly (`unsafe extern "C"`), which is
  also what the C source literally does.

After the fix all four rows produce `SIGSEGV` from both libraries in **both**
profiles. The release profile already matched before the fix, which is why a
release-only test run would have missed this.

### Mutation check (is the suite vacuous?)

The Rust source was deliberately broken, one change at a time, to confirm the
suite actually discriminates. Behaviour-changing mutations are all caught;
semantics-preserving ones correctly stay green.

| mutation | detected |
|----------|----------|
| `sum > 0.0f32` → `sum >= 0.0f32` | yes — 10 tests fail |
| `else if dest != src` → `else if true` | yes — 6 tests fail |
| accumulate in `f64`, round at the end | yes — 15 tests fail |
| accumulate in reverse index order | yes — 15 tests fail |
| off-by-one in the store index | yes — 21 tests fail (guard bands) |
| `memset` fill byte `0` → `1` | yes — 9 tests fail |
| `1.0f32 / x.sqrt()` → `x.sqrt().recip()` | no — `f32::recip` *is* `1.0/self`, bit-identical |
| `sum` initialised to `-0.0` / `MIN_POSITIVE*0.0` | no — `-0.0 + x*x == x*x` and `-0.0 > 0.0` is false |
| clamp the huge `memset` length to `isize::MAX` | no — both lengths fault on the same first unmapped page, so the effect is indistinguishable |

### C ground truth is optimisation-invariant

`c_src/src/lib.c` was additionally compiled at `-O0`, `-O1`, `-O2` and `-O3`
and all four agree with each other and with both Rust profiles bit-for-bit over
20 000 randomised inputs. Only `-Ofast` diverges (`-ffast-math` substitutes an
approximate `rsqrt` for `1.0f/sqrtf`), which is a non-IEEE mode and not the
build defined by `c_src/CMakeLists.txt`.
