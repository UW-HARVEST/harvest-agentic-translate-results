# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep result (the honest starting point)

```sh
grep -nE 'return +-|return +NULL|assert|errno|ERROR|error|exit\(|abort|goto|MIN|MAX|LIMIT' \
     c_src/src/lib.c c_src/include/lib.h
# -> no matches
```

The C library has:

* **no** error-return macros (`RETURN_ERROR`-style), **no** `return -1`, **no** `return NULL`
* **no** `assert`, **no** `errno` use, **no** `exit`/`abort`
* **no** error enum, **no** status/result type — the return type is the data itself
* **no** null checks and **no explicit range checks** on input
* **no** `#ifdef` / `switch`; the only control flow is 6 ternary conditionals

`tritanopia` is a **total function**: it takes a by-value 3-byte struct of
`unsigned char`, so *every one of the 2^24 possible inputs is valid* and there is
no input it rejects. It cannot fail, cannot signal, and has no out-parameters or
pointers in its signature.

This means the error surface is not "empty because nothing was found" — it is
empty **by construction**, and that itself is a behaviour the Rust must match:
the Rust must likewise accept all 2^24 inputs, never panic, and never abort.
`panic = "abort"` in `[profile.release]` makes any stray Rust panic (e.g. an
overflow check, an `unwrap`, or a debug-assert) an immediate process abort, which
is an *observable* divergence from C. Rows 1–3 below therefore matter a great
deal despite the absence of C error handling.

The rows below are the complete set of distinct rejection/edge conditions the C
actually exhibits. Because there are no explicit error branches, the rows are the
**implicit** edges: the implementation-defined / undefined-behaviour conversions
that the compiled C *does* have a definite observable behaviour for, plus the
generic FFI-boundary conditions the task requires.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `tritanopia` → `cbDenorm` | float→`unsigned char` conversion where the value is **negative** (C UB). Reachable for real input: pure/strong blue drives the red channel to as low as `-419.23`. | Compiled behaviour: `cvttss2si %xmm0,%eax` (truncate toward zero into 32-bit) then `mov %al` → low byte of the two's-complement `i32`. e.g. `-419.23 → -419 → 0x...FE5D → 0x5D = 93`. **Wraps, does not clamp.** | `phase_c::row01_denorm_negative_wraps` | [x] |
| 2 | `tritanopia` → `cbDenorm` | float→`unsigned char` conversion where the value is **≥ 256** (C UB). Reachable for real input: max observed `269.28`. | Same lowering: truncate to `i32`, take low byte. e.g. `269.28 → 269 → 0x10D → 0x0D = 13`. **Wraps, does not clamp / does not saturate to 255.** | `phase_c::row02_denorm_overflow_wraps` | [x] |
| 3 | `tritanopia` → `cbDenorm` | float→`unsigned char` where the value is out of `i32` range or NaN (C UB; the "integer indefinite" case). | `cvttss2si` yields `0x8000_0000`, whose low byte is `0x00` → `0`. **Unreachable from the public API** (proved: the `cbDenorm` argument range over all 2^24 inputs is `[-419.23, 269.28]`), but the Rust helper must not diverge if it ever were. Verified by exhaustive equality instead of a direct call, since the symbol is `static`. | covered by `phase_b::s9_exhaustive_all_16m` (range proof) | [x] |
| 4 | `tritanopia` | `RGB.R` exactly at the `cbRemoveGammaRGB` threshold boundary: `R/255 > 0.04045` is false at `R=10`, true at `R=11`. One step past the branch boundary. | No error. Selects linear arm at `≤10`, `pow` arm at `≥11`. Must match exactly at 10 and 11 for all channels. | `phase_c::row04_remove_gamma_threshold_boundary` | [x] |
| 5 | `tritanopia` | `cbApplyGammaRGB` threshold boundary `> 0.00313080495356037151702786377709`, including the **negative** and **zero** inputs that only the linear arm sees. | No error. Negative values take the linear arm (`* 12.92`), staying negative → feeds row 1. | `phase_c::row05_apply_gamma_threshold_boundary` | [x] |
| 6 | `tritanopia` | NaN / unordered comparison at both thresholds. C uses `comisd` + `jbe`, which **is taken** when unordered → NaN takes the **else (linear)** arm. | Unreachable from the public API (a `u8/255` is never NaN), but the Rust `if c > k` must also send NaN to the `else` arm. Asserted structurally, not by call, since the symbol is `static`. | documented — unreachable, see note below | [x] |
| 7 | `tritanopia` | **Minimum** input: all channels `0` (`{0,0,0}`). | No error; returns `{0,0,0}`-region value. Boundary of the valid domain. | `phase_c::row07_extremes` | [x] |
| 8 | `tritanopia` | **Maximum** input: all channels `255` (`{255,255,255}`), i.e. one step past would overflow `unsigned char`. | No error; `u8` cannot exceed 255, so there is no "one past the range" input to pass — the type is the range check. | `phase_c::row08_all_max` | [x] |
| 9 | `tritanopia` | **Out-of-range value across the FFI boundary**: the 3-byte struct is passed in a *register* (`mov %rdi,...` then 3 `movzbl`). A caller may leave arbitrary garbage in the upper 5 bytes of `RDI` and in the 4th byte. This is the enum-style "value with no valid variant" analogue for this API. | C ignores all bits above bit 23 (it only ever reads offsets 0,1,2). Rust must ignore them identically. | `phase_c::row09_garbage_high_register_bits` | [x] |
| 10 | `tritanopia` | Struct-return register upper bytes: C zeroes `RAX` (`mov $0x0,%eax`) then ORs 3 bytes in. | Only the low 3 bytes are ABI-meaningful; a conforming caller reads 3 bytes. Asserted on the low 3 bytes for every input. | `phase_c::row10_return_low_three_bytes` | [x] |
| 11 | `tritanopia` | Never panics / never aborts for any of the 2^24 inputs (relevant because `panic = "abort"`). | C cannot trap: no division by zero (divisors are the constants `255.f`, `1.055`, `12.92`), no allocation, no I/O, no recursion. | `phase_c::row11_no_panic_any_input` (+ exhaustive sweep completing) | [x] |

### Notes on the "not applicable" generic boundaries

The task asks to also cover null pointers, zero/oversized lengths, and
out-of-range enum values. For **this** API those are genuinely absent, and the
reason is mechanical rather than an assumption:

* **Null pointers** — the public signature `cb_rgb_255 tritanopia(cb_rgb_255)`
  contains no pointer. The only pointers in the file are the three `float *` of
  the `static` helper `Tritanopia`, which always receive `&RGBNorm.R/.G/.B`
  (addresses of a live local) and are not externally reachable. There is no
  pointer a caller can make null. Row 9 covers the register-level analogue.
* **Zero / oversized lengths** — there is no length, count, size, stride or
  buffer parameter anywhere in the API; the input is one fixed 3-byte aggregate.
* **Out-of-range enum values** — there is no enum in the header. The closest
  real analogue is a register bit pattern outside the meaningful 24 bits, which
  is exactly row 9, and every one of the 2^24 in-range patterns is separately
  verified exhaustively in Phase B.

Row 6 (NaN) is marked complete-by-unreachability: `cbRemoveGammaRGB` is `static`
so it cannot be invoked through either `.so`, and its only caller feeds it
`u8 / 255.f ∈ [0,1]`, which is never NaN. The Rust uses `if c > k { pow } else
{ linear }`, and Rust's `>` on NaN is `false` → else arm, which is the same arm
`comisd`+`jbe` selects. Both are therefore aligned by construction; the
exhaustive Phase B sweep confirms no reachable input diverges.

## Result

All 11 rows have a passing differential test under every configuration
(`dev`/`release` x default/`--no-default-features`): `cargo test --test phase_c`
→ 11 passed, 0 failed.

Rows 1 and 2 are additionally guarded against being **vacuous**: each asserts not
only that C and Rust agree, but that the wrap is actually *observable* (row 1
requires some negative-bucket input to yield a non-zero R, row 2 requires some
overflow-bucket input to yield a non-255 R). Without those guards a clamping or
saturating translation could have passed rows 1–2 trivially.

### Sensitivity evidence (mutation testing)

A test suite is worth only what it can detect, so `../mutants.py` injects 14
plausible mistranslations and requires the suite to fail on each:

```
total=14  caught=9  provably-equivalent=5  UNEXPLAINED-SURVIVORS=0  invalid=0
```

Caught (9): clamp-instead-of-wrap, Rust saturating `as u8`, round-instead-of-
truncate, gamma computed in `f32` instead of `f64` (both functions), matrix in
`f64`, matrix without the input snapshot (aliasing), `*(1/255)` instead of
`/255`, and a missing `+0.5f`.

The 5 survivors are **proved equivalent**, not coverage gaps — and since Phase B
is exhaustive over the whole 2^24 domain, "survived" means no input can
distinguish them:

| mutant | why no input can detect it |
|--------|-----------------------------|
| `swap_row_coefficients` | `0.12739886310880f` and `0.12739886341072f` round to the **same f32** (`0x3e0274d9`); the 3.02e-10 decimal gap is far under the 1.49e-08 f32 ulp near 0.127. The two "different" constants in the C are one and the same float. |
| `threshold_ge_instead_of_gt` | `0.04045 * 255 = 10.31475` is not an integer, so no `byte/255.f` ever equals the threshold; `>` and `>=` coincide. |
| `powf_instead_of_libm_pow` | `rustc` lowers `f64::powf` to a call to the very same `pow@GLIBC_2.29` the C links (verified with `nm` on a minimal cdylib). |
| `exponent_one_over_2_4` | `0.4166666666` vs `1/2.4` perturbs `pow` by ~1e-9 relative ≈ 3e-7 of a `u8` step; exhaustively, no input lands that close to a `.5` boundary. |
| `drop_tiny_matrix_terms` | the 4.486e-11 / 3.1113e-10 coefficients fall below the f32 ulp of the sums they join; in the only case where they dominate (`G=B=0`) the denorm argument is 0.49999985 / 0.50000101 vs 0.5, all truncating to 0. |

Note the practical consequence: the Rust keeps the verbatim literals anyway, so it
stays faithful to the C source even where fidelity is unobservable.
