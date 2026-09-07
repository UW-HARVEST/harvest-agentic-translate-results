# Phase A.2 — Error-surface table

## Mechanical derivation

Every construct that could reject input was grepped for across the whole C
subtree (`c_src/src`, `c_src/include`):

```sh
grep -nE 'return|assert|NULL|error|ERROR|if|switch|#if|#ifdef|<|>|==|!=|malloc|free' -r src include
```

Complete output — the only four hits are the four unconditional `return`
statements that carry the computed result:

```
src/lib.c:5:    return v;
src/lib.c:9:    return lm_v2(a.x - b.x, a.y - b.y);
src/lib.c:13:    return a.x * b.x + a.y * b.y;
src/lib.c:28:    return lm_v2(u, v);
```

Consequently the library has:

* **0** error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, …)
* **0** `assert` / `static_assert`
* **0** error enums or status codes — the sole entry point returns `lm_vec2`
  by value, so there is no channel through which an error could be reported
* **0** explicit range checks, null checks, or min/max constants
* **0** `if` / `switch` / `?:` / `#ifdef` — the exported function is a single
  straight-line basic block (confirmed in the disassembly: no branch
  instruction of any kind between `<to_barycentric>` and its `ret`)
* **0** pointer parameters — all four parameters and the return value are
  `lm_vec2` passed **by value** (one SSE eightbyte under the SysV x86-64 ABI),
  so "pass a null pointer" is not an expressible input for this API
* **0** enum parameters — so there is no out-of-range-enum input either

The error surface is therefore *empty by construction*. Nothing is rejected;
every one of the 2^256 possible input bit patterns is accepted and produces
some `lm_vec2`. The only "failure-like" outcomes are IEEE-754 exceptional
*results* (non-finite output), which the C produces silently rather than
signalling. Those are the rows below: they are the conditions a caller would
normally call errors, enumerated so that both implementations are proven to
produce the *same* non-finite bit pattern rather than merely "both failing".

## Rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `to_barycentric` | Degenerate triangle, coincident `p1 == p2` (so `v1 == 0`) ⇒ `dot11 == 0`, `dot01 == 0` ⇒ `denom = dot00*0 - 0*0 == +0.0` ⇒ `invDenom = 1.0f/+0.0 = +inf` | no error code; returns `u`/`v` built from `+inf`: `u = (0*dot02 - 0*dot12)*inf = (+0 - +0)*inf = +0*inf = NaN`, `v = (dot00*dot12 - 0*dot02)*inf = ±inf` (or NaN if that numerator is `±0`). Bit pattern must match exactly. |
| 2 | `to_barycentric` | Degenerate triangle, coincident `p1 == p3` (so `v0 == 0`) ⇒ `dot00 == 0`, `dot01 == 0` ⇒ `denom == +0.0` ⇒ `invDenom = +inf` | as row 1 with the roles of `u`/`v` exchanged; no error signalled |
| 3 | `to_barycentric` | Degenerate triangle, all three vertices coincident `p1 == p2 == p3` ⇒ every dot product `+0.0` ⇒ `denom = +0.0` ⇒ `invDenom = +inf`, both numerators `+0.0` | `u = v = +0.0 * +inf = NaN` (specific quiet-NaN bit pattern) |
| 4 | `to_barycentric` | Degenerate triangle, collinear but distinct vertices (`v0` parallel to `v1`, e.g. `p1=(0,0)`, `p2=(1,1)`, `p3=(2,2)`) ⇒ Cauchy–Schwarz equality ⇒ `dot00*dot11 - dot01*dot01 == +0.0` exactly ⇒ `invDenom = +inf` | non-finite `u`/`v` (`±inf` or NaN per numerator sign / zero-ness); no error code |
| 5 | `to_barycentric` | Near-degenerate triangle where catastrophic cancellation drives the denominator **negative**: `dot01*dot01` rounds above `dot00*dot11`, so `denom < 0` even though the exact value (the squared cross product) is non-negative. When additionally `|denom| < 1/FLT_MAX ~ 2.9e-39`, `invDenom = 1.0f/denom = -inf`. Reachability was established mechanically: an 8M-sample sweep of near-parallel `v0`/`v1` over scale exponents `2^-30 .. 2^30` produced 1,313,005 negative denominators, of which 107,365 gave `invDenom = -inf` (example `v0 = (0xb1672278, 0xb230de6e)`, `v1 = (0x3165309c, 0x322f617c)` => `denom = 0x80200000`) | negative finite `invDenom`, or `-inf`; the **sign** of the resulting infinities / finite values must match. Note that `denom == -0.0` is *unreachable*: `dot00`, `dot11` and `dot01*dot01` are each `>= +0.0` (sums and squares of reals), so `t1 = dot00*dot11 >= +0.0` and `t2 = dot01*dot01 >= +0.0`, and `t1 - t2` can only be `+0.0` when both are zero -- confirmed by the same sweep (0 occurrences of `-0.0`). |
| 6 | `to_barycentric` | Denominator underflows to `+0.0` from non-zero products (very small coordinates, e.g. components ≈ `1e-23f`, so `dot00*dot11` and `dot01*dot01` are both subnormal/zero and their difference flushes to `+0.0`) ⇒ `invDenom = +inf` | non-finite result; no error code, no errno, no exception raised |
| 7 | `to_barycentric` | Denominator overflows to `+inf` (huge coordinates, e.g. `1e30f`, so `dot00*dot11` overflows) ⇒ `invDenom = 1.0f/+inf = +0.0` | finite-or-NaN `u`/`v`: numerator is `inf - inf = NaN` in general ⇒ `NaN * 0 = NaN`; exact bit pattern must match |
| 8 | `to_barycentric` | Denominator is `-inf` (huge coordinates with `dot01*dot01` overflowing) ⇒ `invDenom = -0.0` | as row 7 with the sign of any finite/zero result flipped |
| 9 | `to_barycentric` | Any single input component `= +inf` | subtraction produces `±inf`; dot products produce `inf` or `inf - inf = NaN`; propagates to `u`/`v`. No rejection. |
| 10 | `to_barycentric` | Any single input component `= -inf` | as row 9; sign-dependent NaN/inf pattern must match |
| 11 | `to_barycentric` | `p == p1` exactly (so `v2 == 0`) ⇒ `dot02 == dot12 == +0.0` | `u = (dot11*0 - dot01*0) * invDenom = +0.0 * invDenom`; `v` likewise. Signed zero of the result must match (`+0.0` vs `-0.0`). |
| 12 | `to_barycentric` | Input component is a **quiet NaN** (`0x7fc00000`) | result is a quiet NaN with a *specific* payload/sign determined by x86 SSE first-operand-wins propagation. Not an error; bit pattern must match. |
| 13 | `to_barycentric` | Input component is a **signalling NaN** (`0x7f800001`) | C raises no trap (SSE MXCSR invalid is masked): the sNaN is *quieted* to `0x7fc00001` and propagated. Rust must quiet identically, not pass the sNaN through. |
| 14 | `to_barycentric` | **Two different** NaN payloads reach the same commutative operation (e.g. `p3.x = NaN_a`, `p2.x = NaN_b`) | x86 returns the NaN of the *destination* (first) operand, so which payload survives depends on the exact instruction operand order GCC emitted. This is the sharpest error-path discriminator in the whole library. |
| 15 | `to_barycentric` | Input component is a **negative NaN** (sign bit set, `0xffc00000`) | sign bit is preserved through propagation; result NaN sign must match |
| 16 | `to_barycentric` | Input component is a **subnormal** (`0x00000001` … `0x007fffff`) | no denormals-are-zero / flush-to-zero behaviour is enabled by the C build, so subnormals participate normally; results must match bit-for-bit (this catches a Rust build that enabled FTZ/DAZ) |
| 17 | `to_barycentric` | Input component is `-0.0` | signed zero propagates through `sub`/`mul`/`add` per IEEE-754; the sign bit of a zero result must match |
| 18 | `to_barycentric` | All 8 components are the maximum finite float `0x7f7fffff` | intermediate overflow to `inf`, `inf - inf = NaN`; must match |
| 19 | `to_barycentric` | Structs whose padding/upper bits are non-zero — i.e. an `lm_vec2` passed with garbage in the upper 64 bits of the XMM register | `lm_vec2` is exactly 8 bytes with no padding (`sizeof == 8`, `alignof == 4`), and the ABI passes it in the low eightbyte, so the upper bits are ignored by both. Verified by asserting Rust `size_of`/`align_of` equal the C `sizeof`/`_Alignof`. |
| 20 | `to_barycentric` | Called concurrently from many threads | the function is pure, has no static/global state (confirmed: `.bss` is empty apart from CRT, `.data.rel.ro` holds no library state), so it is reentrant. Rust must not introduce any shared mutable state. |

## Gate

- [x] Row 1 — `error_paths::row01_coincident_p1_p2`
- [x] Row 2 — `error_paths::row02_coincident_p1_p3`
- [x] Row 3 — `error_paths::row03_all_coincident`
- [x] Row 4 — `error_paths::row04_collinear_distinct`
- [x] Row 5 — `error_paths::row05_negative_denominator`
- [x] Row 6 — `error_paths::row06_denominator_underflow`
- [x] Row 7 — `error_paths::row07_denominator_overflow_pos`
- [x] Row 8 — `error_paths::row08_denominator_overflow_neg`
- [x] Row 9 — `error_paths::row09_component_pos_inf`
- [x] Row 10 — `error_paths::row10_component_neg_inf`
- [x] Row 11 — `error_paths::row11_p_equals_p1`
- [x] Row 12 — `error_paths::row12_quiet_nan`
- [x] Row 13 — `error_paths::row13_signalling_nan`
- [x] Row 14 — `error_paths::row14_two_distinct_nan_payloads`
- [x] Row 15 — `error_paths::row15_negative_nan`
- [x] Row 16 — `error_paths::row16_subnormal`
- [x] Row 17 — `error_paths::row17_negative_zero`
- [x] Row 18 — `error_paths::row18_all_max_finite`
- [x] Row 19 — `error_paths::row19_struct_layout_and_upper_bits`
- [x] Row 20 — `error_paths::row20_thread_safety`

## Notes on generic boundaries required by Phase C

* **Null pointers** — not expressible: `lib.h` declares no pointer type at all.
  `error_paths::row19_struct_layout_and_upper_bits` asserts this mechanically by
  reading `c_src/include/lib.h` and failing if it ever contains `*`.
* **Zero and oversized lengths** — not expressible: there is no length, count or
  size parameter anywhere in the API.
* **Out-of-range enum values across the FFI boundary** — not expressible: the API
  declares no enum. The same test fails if `lib.h` ever gains the `enum` keyword.
  The nearest analogue for this API is a `float` argument whose bit pattern has no
  "normal" interpretation, and *every* one of the 2^32 patterns is a legal input;
  rows 12–18 cover them class by class and `CONFIGS.md` row 25 plus
  `symbol_parity::heavy_uniform_fuzz` cover them jointly with uniformly random
  bits (4,000,000 cases per run by default, raised to 50,000,000 during the
  mutation study).
* **One step past a documented valid range** — there is no documented valid
  range; the boundary values that the *arithmetic* distinguishes (`f32::MAX`,
  `f32::MIN_POSITIVE`, the largest subnormal `0x007fffff`, the smallest subnormal
  `0x00000001`, `±0.0`, `±inf`) are each covered by rows 16–18 and by
  `CONFIGS.md` rows 12, 16, 17.
