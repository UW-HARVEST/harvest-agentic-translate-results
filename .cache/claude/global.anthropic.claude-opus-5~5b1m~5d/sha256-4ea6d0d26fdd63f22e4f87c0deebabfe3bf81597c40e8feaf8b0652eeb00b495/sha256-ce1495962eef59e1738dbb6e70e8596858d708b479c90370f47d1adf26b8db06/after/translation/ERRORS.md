# ERRORS.md — error-surface table (Phase A / gate for Phase C)

## Mechanical derivation

Every rejection path in the C was found by grepping the whole of `c_src`
(2 files, 49 lines total):

```
$ grep -n 'return' src/lib.c include/lib.h
src/lib.c:14:        return;                 <- bare `return;` from a void function

$ grep -niE 'assert|null|errno|error|exit|abort' src/lib.c include/lib.h
(no matches)

$ grep -niE 'RETURN_ERROR|return *-|return *NULL|enum' src/lib.c include/lib.h
(no matches)
```

**Result of the grep: the C library has NO error-reporting surface at all.**

* `hsl_to_rgb` returns `void` — there is no error code, no sentinel, no
  out-parameter status, no `errno` use, and no global error state.
* There is not a single `assert`, range check, null check, size check or
  capacity check. `dest` and `src` are dereferenced unconditionally.
* There are no enums anywhere in the public header, so there is no
  "out-of-range enum value" to pass across the FFI boundary (the only inputs
  are two pointers and the three `float`s they point at). Every one of the
  2^32 bit patterns of a `float` is an *accepted* input; none is rejected.
* The only named constants are the floating-point literals
  `0.0f 0.5f 1.0f 2.0f 60.0f 120.0f 180.0f 240.0f 300.0f 360.0f` and the
  integer `2`; they are **sector boundaries**, not validity limits — a hue
  outside `[0, 360)` is not rejected, it silently takes a different branch.

Because the C never rejects anything, the "error surface" that has to be
verified is the set of **degenerate / exceptional conditions where the C takes a
non-obvious exit or produces a non-finite result**, plus the generic C-API
boundary conditions the instructions call out (null pointers, out-of-range
values, values one step past a documented range). Those are the rows below. Each
row states the *exact* C result so the differential test can assert equality of
the actual result, not merely "both did something".

Notation: `h = src[0]`, `s = src[1]`, `l = src[2]`; outputs are compared as raw
`u32` bit patterns (so `+0.0` vs `-0.0` and distinct NaN payloads are
distinguished).

**Note on generated NaNs.** Every NaN the *hardware* creates for an invalid
operation on x86 is the "QNaN floating-point indefinite" `0xffc00000` — the sign
bit is **set**, unlike Rust's `f32::NAN` (`0x7fc00000`). Verified directly against
the C toolchain:

```
fmodf(+Inf,2) = 0xffc00000     Inf - Inf = 0xffc00000
fmodf(-Inf,2) = 0xffc00000     0  * Inf  = 0xffc00000
Inf / Inf     = 0xffc00000     0  /  0   = 0xffc00000
```

A translation that synthesised NaNs with `f32::NAN` instead of letting the
hardware produce them would be wrong in the sign bit on every one of these paths.

Derived quantities:

```
c = (1.0f - fabsf(2.0f*l - 1.0f)) * s
m = 1.0f * (l - 0.5f*c)
x = c * (1.0f - fabsf(fmodf(h/60.0f, 2) - 1.0f))
```

## The table

| #    | function | trigger (the exact invalid input/condition) | expected C result |
|------|----------|---------------------------------------------|-------------------|
| E1  | `hsl_to_rgb` | `s == 0.0f` exactly (`src[1]` is `+0.0f`) — the only early `return` in the library (`lib.c:10-15`) | `dest[0]=dest[1]=dest[2]=l` bit-for-bit (**`l` copied verbatim**, incl. NaN payload/sign, Inf, subnormals); `c`/`m`/`x` are never computed, so `fmodf` is never called and no FP exception is raised; returns `void` |
| E2  | `hsl_to_rgb` | `s == -0.0f` (`0x80000000`) — `-0.0f == 0` is *true* in C, so this hits the same early return, not the general path | identical to E1: three copies of `l` |
| E3  | `hsl_to_rgb` | `s` is NaN (any payload/sign) — both `ucomiss` comparisons fail (unordered ⇒ `jp`/`jne` taken), so the `s == 0` early-out is **not** taken | falls through to the general path; `c = (…) * NaN` ⇒ quiet(NaN) propagated into `c`, `m`, `x`; the hue branch is still selected normally and the three stores are NaN-valued |
| E4  | `hsl_to_rgb` | `h` in the "hole" `[120.0f, 180.0f)` — created by the typo at `lib.c:27` (`h < 120.0f && h < 180.0f` instead of `h >= 120.0f && h < 180.0f`). This whole 60° sector is *silently* unreachable for its intended branch | falls all the way through to the final `else`: `dest[0]=dest[1]=dest[2]=m` (grey). **NOT** the cyan sector. Must be reproduced, not fixed. |
| E5  | `hsl_to_rgb` | `h < 0.0f` (any negative, including `-FLT_MIN`, `-FLT_MAX`, `-Inf`) — one step past the documented low end of the hue range | takes branch 3 (`lib.c:27`) because `h < 120 && h < 180` is true: `dest = {m, c+m, x+m}`. `x` uses `fmodf(h/60,2)` which is **negative** for negative `h`, so `1-\|fmod-1\|` is negative ⇒ `x` has the opposite sign of the positive-hue case. |
| E6  | `hsl_to_rgb` | `h >= 360.0f` (e.g. `360.0f`, `nextafterf(360,inf)`, `1e30f`, `FLT_MAX`, `+Inf`) — one step past the documented high end; no wrap-around is performed | final `else`: `dest[0]=dest[1]=dest[2]=m` (grey). Note `h=360.0f` itself is grey: the last test is `h < 360.0f`, strict. |
| E7  | `hsl_to_rgb` | `h` is NaN (any payload/sign) — every one of the 12 `comiss` tests is unordered, so **all six** hue branches fail | final `else`: `dest[0]=dest[1]=dest[2]=m`. `x` is computed (and is NaN) but discarded. `comiss` on a NaN raises `#IA` in the C; the result written is `m`, which is NaN only if `l`/`s` make it so. |
| E8  | `hsl_to_rgb` | `h = ±Inf` ⇒ `h/60 = ±Inf` ⇒ `fmodf(±Inf, 2)` is the libm domain error `(x*y)/(x*y)` = `Inf/Inf` | `fmodf` returns the **x86 indefinite** quiet NaN `0xffc00000` (sign bit SET — *not* `0x7fc00000`; verified against glibc), `invalid` raised. `fabsf` later clears that sign, so `x` ends up `0x7fc00000`. `h=+Inf` then takes the final `else` ⇒ grey `m`; `h=-Inf` takes branch 3 ⇒ `{m, c+m, NaN}`. This is the row that exercises the **glibc-`fmodf` vs `compiler_builtins`-`fmodf`** difference documented in `SYMBOLS.md`. |
| E9  | `hsl_to_rgb` | `l = ±Inf` with finite non-zero `s` ⇒ `2*l-1 = ±Inf`, `fabsf` ⇒ `+Inf`, `1-Inf = -Inf`, `c = -Inf*s` (sign of `s`), `m = l - 0.5*c` ⇒ `Inf - (∓Inf)` may be `Inf-Inf` | `Inf - Inf` produces the **x86 indefinite** quiet NaN `0xffc00000` (sign bit set) in `m`; propagates to every store. Covers the `invalid` operation reached with *valid* (non-rejected) input. |
| E10 | `hsl_to_rgb` | `1.0f - fabsf(2*l-1) == 0` (i.e. `l == 0.0f` or `l == 1.0f`) **and** `s = ±Inf` ⇒ `c = 0 * ±Inf` | `c` = the **x86 indefinite** quiet NaN `0xffc00000` (sign bit set); then `m = l - 0.5*NaN` = NaN, `x = (…)*NaN` = NaN ⇒ all three stores NaN. Second `invalid`-from-valid-input case. |
| E11 | `hsl_to_rgb` | `src == NULL` | **Undefined behaviour — the C performs no null check** (`mov -0x30(%rbp),%rax; movss (%rax),%xmm0` dereferences unconditionally). Both libraries must fault identically: `SIGSEGV`, no message, no error code. Verified in a forked child process. |
| E12 | `hsl_to_rgb` | `dest == NULL` (with valid `src`, `s != 0`) | **Undefined behaviour — no null check.** Both libraries must fault identically with `SIGSEGV` on the first store. Verified in a forked child process. |
| E13 | `hsl_to_rgb` | `dest == NULL` **and** `s == 0` (the early-return path stores too) | **UB, `SIGSEGV`** in both — proves the early-return path also has no null guard. |
| E14 | `hsl_to_rgb` | "oversized length": caller supplies a buffer *larger* than 3 floats, or a `src` array longer than 3 | the C reads **exactly** `src[0..3]` and writes **exactly** `dest[0..3]`; bytes at index ≥ 3 (and at negative offsets) must be left untouched. Asserted with canary padding on both sides of `dest`. |
| E15 | `hsl_to_rgb` | "zero length" / partial buffer: only 3 floats exist and nothing more | no out-of-bounds access; same 3 stores. (Same assertion as E14 with a tight allocation; there is no length argument to make zero, so this is the only meaningful reading of "zero length" for this API.) |
| E16 | `hsl_to_rgb` | full aliasing `dest == src` | the C copies `h`, `s`, `l` into locals *before* any store (`lib.c:6-8`), so aliasing is well defined and lossless; the Rust must read all three before storing too. Result identical to the non-aliased call. |
| E17 | `hsl_to_rgb` | partial overlap `dest == src + 1` and `dest == src - 1` | same as E16: all reads precede all writes, so the result equals the non-aliased call written at the overlapping offset. Detects a translation that interleaved reads and writes. |
| E18 | `hsl_to_rgb` | a signalling NaN (`0x7f800001`, `0xff800001`) in `h`, `s` or `l` | every SSE arithmetic op **quiets** it (sets bit 22) while keeping sign and payload; a value merely *copied* (the `s == 0` path copying `l`) is **not** quieted. The Rust `quiet()` helper must reproduce exactly this asymmetry. |
| E19 | `hsl_to_rgb` | subnormal / `FLT_MIN` / `FLT_TRUE_MIN` in any component (values one step past the normal-number range) | no flush-to-zero anywhere (neither library sets `MXCSR.FTZ`/`DAZ`); exact IEEE-754 single-precision result. |
| E20 | `hsl_to_rgb` | every bit pattern class at once: `h`, `s`, `l` each drawn independently from the full 32-bit space (fuzz) | whatever the C produces, bit-for-bit. This is the catch-all row that guarantees no rejection path was missed by the grep above. |

## Status

| row | test | status |
|-----|------|--------|
| E1  | `errors::e1_s_is_positive_zero`            | [x] |
| E2  | `errors::e2_s_is_negative_zero`            | [x] |
| E3  | `errors::e3_s_is_nan`                      | [x] |
| E4  | `errors::e4_hue_hole_120_to_180`           | [x] |
| E5  | `errors::e5_negative_hue`                  | [x] |
| E6  | `errors::e6_hue_at_or_above_360`           | [x] |
| E7  | `errors::e7_hue_is_nan`                    | [x] |
| E8  | `errors::e8_hue_is_infinite`               | [x] |
| E9  | `errors::e9_lightness_is_infinite`         | [x] |
| E10 | `errors::e10_zero_times_infinite_chroma`   | [x] |
| E11 | `errors::e11_null_src_faults_in_both`      | [x] |
| E12 | `errors::e12_null_dest_faults_in_both`     | [x] |
| E13 | `errors::e13_null_dest_zero_sat_faults`    | [x] |
| E14 | `errors::e14_no_out_of_bounds_write`       | [x] |
| E15 | `errors::e15_tight_buffer_no_overrun`      | [x] |
| E16 | `errors::e16_full_aliasing_dest_eq_src`    | [x] |
| E17 | `errors::e17_partial_overlap`              | [x] |
| E18 | `errors::e18_signalling_nan`               | [x] |
| E19 | `errors::e19_subnormals`                   | [x] |
| E20 | `errors::e20_full_bit_pattern_fuzz`        | [x] |

## Addendum — rows found by Phase C that the first pass of the table missed

Writing the tests above surfaced two *additional* rejection-adjacent behaviours
that belong in this table. They are recorded here rather than silently fixed,
because each one was a genuine divergence in the Rust that had to be corrected.

| #    | function | trigger (the exact invalid input/condition) | expected C result |
|------|----------|---------------------------------------------|-------------------|
| E21 | `hsl_to_rgb` | `h` is a **quiet** NaN (with `s != 0`, so the dispatch chain is reached) — the C compiles `h >= 0.0f` / `h < 60.0f` to **`comiss`**, the *signalling* compare, which raises the invalid-operation exception even for a quiet NaN. Rust's `>=`/`<` lower to the *quiet* `ucomiss`, which does not. | `fetestexcept(FE_INVALID) != 0` after the call. **Found divergent** (4430 failing cases): the Rust left `FE_INVALID` clear. Fixed by raising it explicitly for a NaN hue. |
| E22 | `hsl_to_rgb` | a **signalling** NaN in `h` or `l`, where the first arithmetic instruction that touches it (`divss h,60` / `mulss 2,l`) must raise `FE_INVALID`. A translation that *short-circuits* the arithmetic when an operand is a NaN never executes that instruction and so never raises the flag. | `fetestexcept(FE_INVALID) != 0`. Fixed by always performing the hardware operation (behind `black_box`) and overriding only the resulting *value*. |
| E23 | `hsl_to_rgb` | `dest`/`src` null, **in a `debug` (`[profile.dev]`) build** — Rust's debug-assertion-gated UB checks turn the null dereference into a non-unwinding panic (`SIGABRT`) instead of the `SIGSEGV` the unchecked C produces. | `SIGSEGV`. **Found divergent**: `SIGABRT` (signal 6) vs `SIGSEGV` (signal 11). Fixed by setting `debug-assertions = false` / `overflow-checks = false` in `[profile.dev]`, so the dev artifact is as unchecked as the C. |

| row | test | status |
|-----|------|--------|
| E21 | `fenv::fp_status_flags_match` | [x] |
| E22 | `fenv::fp_status_flags_match` | [x] |
| E23 | `errors::e11/e12/e13_*` (run against both the debug and the release `.so`) | [x] |
