# ERRORS.md — Phase C: error-surface table

## Mechanical derivation

Every rejection construct was grepped for across the whole C tree
(`c_src/include/lib.h`, `c_src/src/lib.c` — 34 lines total):

```
$ grep -nE 'return -|return NULL|RETURN_ERROR|assert|errno|abort|exit\(|_ERR|ERROR|goto|if|else|switch|\?|<|>|==|!=|&&|\|\||NULL|MIN|MAX|LIMIT' c_src/src/lib.c c_src/include/lib.h
(no matches)
```

**Result: the C library has an empty rejection surface.**
- 0 error-return macros, 0 `return -1`, 0 `return NULL`, 0 error enums.
- 0 `assert`, 0 `abort`, 0 `exit`.
- 0 range checks, 0 null checks, 0 min/max constants.
- 0 `if`/`else`/`switch`/`?:` — `to_barycentric` is straight-line code.
- 0 pointer parameters and 0 enum parameters, so "null pointer" and
  "out-of-range enum value" are **unrepresentable** in this API: all four
  parameters are `lm_vec2` structs passed *by value*, and the only field type
  is `float`, whose every one of the 2^32 bit patterns is a legal input.
- 0 length/count/size parameters, so "zero length" and "oversized length" are
  likewise unrepresentable.

Because the C never rejects anything, "the same error" means **the same
IEEE-754 result bit pattern** for the degenerate inputs that a checking library
*would* have rejected. Each row below is such a condition: it is a distinct
way the computation goes non-finite or otherwise "fails", and the requirement
is that C and Rust return bit-identical `(u, v)`.

The single non-finite generator in the C is line 25:

```c
float invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01);   // no degeneracy check
```

## Error-surface table

All rows are exercised by `tests/differential.rs` (module `phase_c`) and
compared as raw `u32` bit patterns, sign and NaN payload included.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `to_barycentric` | degenerate triangle: all four points coincident (`p1==p2==p3==p`), so `v0=v1=v2=(0,0)` and `denom = 0*0 - 0*0 = +0` | `1.0f/+0.0f = +inf`; `u = (0*0-0*0)*inf = +0*inf = -nan` (hardware default QNaN `0xffc00000`); returns `(0xffc00000, 0xffc00000)` — no error signalled | `e1_all_points_coincident` | [x] |
| E2 | `to_barycentric` | `p2 == p1` (zero-length `v1` edge): `dot11 = dot01 = 0`, so `denom = dot00*0 - 0*0 = ±0` | `invDenom = ±inf`; `u = (0*dot02 - 0*dot12)*inf = ±0*±inf = nan`, `v = (dot00*0 - 0*dot02)*inf = nan`; both NaN, bit-exact match required | `e2_p2_equals_p1` | [x] |
| E3 | `to_barycentric` | `p3 == p1` (zero-length `v0` edge): `dot00 = dot01 = dot02 = 0`, `denom = 0*dot11 - 0 = ±0` | `invDenom = ±inf`, both coordinates NaN; bit-exact match required | `e3_p3_equals_p1` | [x] |
| E4 | `to_barycentric` | collinear (non-coincident) triangle, e.g. `p1=(0,0)`, `p2=(1,1)`, `p3=(2,2)`: Cauchy–Schwarz equality makes `dot00*dot11 - dot01*dot01` mathematically `0`, so `denom` is `±0` or a tiny rounding residue | `invDenom = ±inf` or a huge finite value; `u`/`v` are `±inf`/`nan`/huge accordingly — whatever the C produces, bit-identical | `e4_collinear` | [x] |
| E5 | `to_barycentric` | denominator underflows to `+0` from tiny-but-nonzero edges (e.g. edges scaled by `1e-30f`, so `dot*dot ~ 1e-120` → flushes to `0` in `f32`) | `1.0f/+0.0f = +inf`, then `±0 * inf = nan`; bit-identical | `e5_denominator_underflow` | [x] |
| E6 | `to_barycentric` | denominator overflows to `±inf` from huge edges (e.g. coords `~1e30f`, so `dot00*dot11 ~ 1e120` → `+inf`, and `inf - inf = -nan`) | `denom = -nan` → `invDenom = -nan` → both coordinates NaN with the *first* NaN's payload (SSE `divss dst=1.0f` is non-NaN so the source NaN wins); bit-identical | `e6_denominator_overflow` | [x] |
| E7 | `to_barycentric` | an input coordinate is `+inf`/`-inf` (all 4 points × 2 fields, both signs) | `inf - x = inf`, `inf*inf = inf`, `inf - inf = -nan` etc. propagate with no check; bit-identical | `e7_infinite_coordinates` | [x] |
| E8 | `to_barycentric` | an input coordinate is a **quiet** NaN with a non-default payload (all 4 points × 2 fields, both signs, several payloads) | the payload propagates through `subss`/`mulss`/`addss`/`divss`; which operand's payload survives is fixed by the destination-register choice of the unoptimised C build (see `mul_dst_lhs`/`mul_dst_rhs`/`add_dst_rhs` in `src/lib.rs`); bit-identical including payload | `e8_quiet_nan_payloads` | [x] |
| E9 | `to_barycentric` | an input coordinate is a **signaling** NaN (`exp=0xFF`, `mantissa!=0`, quiet bit clear) | SSE quiets it: result payload = input payload with bit 22 set, sign preserved; bit-identical | `e9_signaling_nan` | [x] |
| E10 | `to_barycentric` | subnormal / smallest-denormal coordinates (`0x00000001`, `0x007fffff`), incl. mixed with normals | products underflow to `±0`; `denom = ±0` → `±inf` → `nan`; bit-identical (no FTZ/DAZ is set by either build) | `e10_subnormal_inputs` | [x] |
| E11 | `to_barycentric` | negative zero coordinates (`-0.0f`), which make `denom = -0.0f` rather than `+0.0f` | `1.0f/-0.0f = -inf` (sign matters); `u`,`v` NaN/`∓inf`; bit-identical **including the sign bit** | `e11_negative_zero` | [x] |
| E12 | `to_barycentric` | "one step past the valid range": the extremal finite floats `±FLT_MAX` (`0x7f7fffff`) and `±FLT_MIN` as coordinates — the API documents no range, so these are the boundary values | `subss` overflows to `±inf` for `FLT_MAX - (-FLT_MAX)`, then `inf`/`nan` propagation; bit-identical | `e12_extremal_finite` | [x] |
| E13 | `to_barycentric` | every field of every argument set to the *same* arbitrary bit pattern, swept over all 256 exponent values (catches any exponent-dependent divergence, incl. the `0x00`/`0xff` boundaries) | whatever the C returns, bit-identical | `e13_exponent_sweep` | [x] |

### Explicitly-not-applicable generic boundaries

| generic C-API boundary | applicable? | why |
|------------------------|-------------|-----|
| null pointer argument | no | no pointer parameters; all four args are by-value structs |
| zero length / oversized length | no | no length, size, count or capacity parameter exists |
| out-of-range enum value across FFI | no | no enum parameter or return type exists |
| value one past a documented valid range | covered by E12/E13 | no range is documented or checked; the float-domain boundaries (`±FLT_MAX`, `±0`, subnormals, `±inf`, NaN) are enumerated instead |
| error code / errno | no | function returns `lm_vec2` by value; no status channel, and it never touches `errno` |
| output-buffer overflow | no | no output buffer; the result is returned in registers |
