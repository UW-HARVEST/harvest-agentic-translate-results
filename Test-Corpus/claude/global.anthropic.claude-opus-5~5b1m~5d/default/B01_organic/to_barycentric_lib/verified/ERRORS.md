# ERRORS.md — Phase A: error-surface table

## Mechanical scan of the C source

Every rejection construct a C library can use was grepped for across the whole
of `c_src` (`src/lib.c`, `include/lib.h`). Result:

| construct grepped | matches in `c_src` |
|-------------------|--------------------|
| `return -1` / `return <int>` (`return[[:space:]]+-?[0-9]`) | 0 |
| `return NULL`      | 0 |
| `RETURN_ERROR` / any error macro | 0 |
| `assert` / `static_assert` / `abort` | 0 |
| `errno`            | 0 |
| `if` / `else`      | 0 |
| `switch` / `case`  | 0 |
| `goto`             | 0 |
| `enum`             | 0 |
| `malloc` / `calloc` / `realloc` / `free` | 0 |
| pointer declarations / `[` array indexing | 0 |
| `MIN` / `MAX` / `LIMIT` / range constants | 0 |
| `#if` / `#ifdef` / `#error` | 0 |

```sh
$ grep -nE 'return[[:space:]]+-?[0-9]|return[[:space:]]+NULL|RETURN_ERROR|assert|errno|\bif\b|\bswitch\b|goto|enum|malloc|calloc|free|MIN|MAX|LIMIT|#if' \
      c_src/src/lib.c c_src/include/lib.h
# (no output)
```

**Conclusion, derived from the source and not from docs:** `to_barycentric` has
**no validation, no branches, no error return channel and no pointer or enum
parameters.** Its signature is

```c
lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
```

— four 8-byte aggregates **by value**, one aggregate returned by value. There is
no `int` status, no out-pointer, no `NULL`-able argument, and no `enum`.
Consequently the *entire* error surface of this library is the set of
**IEEE-754 exceptional conditions** reachable through its unguarded arithmetic,
plus the ABI-level "invalid input" cases that a hostile FFI caller can construct
for a by-value float aggregate (a `float` accepts every one of its 2^32 bit
patterns, so trap representations do not exist — but ±inf, subnormals, and both
quiet and *signalling* NaNs do, and they are real inputs the C handles).

Each row below is one distinct way the C produces a non-finite / "rejected"
result, together with the exact C result. `Q(x)` = `x` with the quiet bit
(`0x0040_0000`) set, sign and payload preserved — the x86 SNaN→QNaN conversion.
`IND` = the x86 "real indefinite" default QNaN produced by an *invalid*
operation on non-NaN operands, i.e. `0xFFC0_0000` (**negative** sign).

The only division in the function is the unguarded reciprocal on
`c_src/src/lib.c:25`:

```c
float invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01);
```

`dot00*dot11 - dot01*dot01` is the squared area (Gram determinant) of the
`(v0, v1)` pair, so it is `0` for every degenerate triangle. There is **no**
degeneracy check, which is the single most important "error" behaviour to
replicate.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `to_barycentric` | **All four points coincident** (`p1==p2==p3==p`, e.g. all-zero). `v0=v1=v2=0` ⇒ all dots `0` ⇒ denom `0-0 = +0.0` ⇒ `invDenom = 1.0f/+0.0 = +inf`; numerators are `+0.0-(+0.0) = +0.0` ⇒ `0.0 * inf` = **invalid op** | `(u,v) = (IND, IND) = (0xFFC0_0000, 0xFFC0_0000)` | `e1_all_points_coincident` |
| E2 | `to_barycentric` | **`p2 == p1`** (`v1 = 0`) but `p3 != p1`. `dot01=dot11=dot12=0`, `dot00>0` ⇒ denom `= dot00*0 - 0*0 = +0.0` ⇒ `invDenom=+inf`; `u`-numerator `= 0*dot02 - 0*0 = ±0.0`, `v`-numerator `= dot00*0 - 0*dot02 = ±0.0` ⇒ `±0.0 * +inf` = invalid | `(IND, IND)` (both lanes; sign of the `0` does not change the indefinite) | `e2_p2_equals_p1` |
| E3 | `to_barycentric` | **`p3 == p1`** (`v0 = 0`) but `p2 != p1`. `dot00=dot01=dot02=0`, `dot11>0` ⇒ denom `+0.0` ⇒ same invalid `0*inf` | `(IND, IND)` | `e3_p3_equals_p1` |
| E4 | `to_barycentric` | **Exactly collinear triangle**, `p3-p1 = t*(p2-p1)` with `t` a power of two and dyadic coords, so no rounding intervenes. Substituting `v0 = t·v1` gives `dot00 = t²·dot11`, `dot01 = t·dot11`, `dot02 = t·(v1·v2)`, so the determinant **and both numerators** are exactly `+0.0` | measured: `(IND, IND)` in **every** case — `1.0f/+0.0 = +inf` then `+0.0 * +inf` = invalid. *Not* `±inf`: the numerators vanish too | `e4_collinear_triangle` |
| E5 | `to_barycentric` | **Rounding-induced zero denominator**: near-collinear input where `dot00*dot11 - dot01*dot01` rounds to `+0.0` while a numerator stays non-zero. This — not E4 — is the case that actually yields infinities. Witness pinned as a literal in the test | `±inf`, sign from the numerator: measured `(+inf, -inf)` for the pinned witness. Reached by ≈0.5 % of a 300 000-case near-collinear sweep | `e5_near_collinear_rounds_to_zero` |
| E6 | `to_barycentric` | **Negative denominator (catastrophic cancellation)**: `dot00*dot11 - dot01*dot01 < 0`, unreachable mathematically (Cauchy–Schwarz) but reachable when `dot00*dot11` rounds down and `dot01*dot01` rounds up ⇒ `invDenom < 0` and both signs flip. Witness pinned as a literal | measured `(-0.0, 0.125)` for the pinned witness — note the **negative zero**, which a non-bitwise comparison would have missed. ≈32 % of the near-collinear sweep has a negative denominator | `e5_near_collinear_rounds_to_zero` |
| E6b | `to_barycentric` | **Denominator exactly `-0.0`** (would give `1.0f/-0.0 = -inf`) | **unreachable**, and the test asserts it: `dot00 = x²+y²` and `dot11` are each `+0.0` or positive (a square is never `-0.0`), so `dot00*dot11 ≥ +0.0`; and `x - y = -0.0` in round-to-nearest only when `x = -0.0, y = +0.0`. Asserted over all 256 signed-zero inputs | `e16_signed_zero` |
| E7 | `to_barycentric` | **Coordinate magnitudes near `FLT_MAX`** (e.g. `3.4e38`): `lm_sub2` overflows to `±inf`, or a squared term in `lm_dot2` overflows ⇒ `dot* = ±inf` | `±inf` propagates; `inf*0`, `inf-inf`, `inf/inf` each ⇒ `IND`; `1.0f/±inf` ⇒ `±0.0` | `e7_overflow_to_infinity` |
| E8 | `to_barycentric` | **`inf - inf` inside `lm_sub2`**: `p3.x = +inf` and `p1.x = +inf` | that component of `v0` is `IND`; NaN then floods every dependent dot product | `e8_inf_minus_inf` |
| E9 | `to_barycentric` | **`0 * inf` inside `lm_dot2`**: one component `±inf`, the paired component `±0.0` | that product is `IND`; `IND + finite = IND` | `e9_zero_times_inf` |
| E10 | `to_barycentric` | **`inf + (-inf)` inside `lm_dot2`'s `addss`** (`a.x*b.x = +inf`, `a.y*b.y = -inf`) | `IND` | `e10_inf_plus_neg_inf` |
| E11 | `to_barycentric` | **`inf / inf`**: `1.0f / inf` is fine (`+0.0`), but `inf/inf` arises via `denom = inf` … `numerator = inf` ⇒ `inf * 0.0` | `IND` where invalid, `±0.0`/`±inf` otherwise — asserted against C | `e7_overflow_to_infinity` |
| E12 | `to_barycentric` | **Underflow to `±0.0` / subnormal**: coordinates at `FLT_MIN`/`FLT_TRUE_MIN` scale ⇒ squares flush to `+0.0` (gradual underflow, then total) ⇒ denom `+0.0` ⇒ `invDenom = +inf` ⇒ `0*inf` | `IND` where invalid; otherwise the (subnormal-precision-lossy) finite value — asserted bit-for-bit | `e12_underflow_subnormal` |
| E13 | `to_barycentric` | **Quiet NaN in any one of the 8 input floats** (8 positions × several payloads) | the QNaN payload propagates; *which* payload survives is fixed by the SSE destination operand at `-O0` (see `CONFIGS.md` C1–C4) | `e13_single_qnan_each_position` |
| E14 | `to_barycentric` | **Signalling NaN** (`0x7F80_0001`-style, quiet bit clear) in any input float — a value a C caller can absolutely pass, and `float` has no trap representation so it is not UB | the SNaN is **quieted** (`\|= 0x0040_0000`) by the first arithmetic op that consumes it; sign and low payload bits are preserved | `e14_single_snan_each_position` |
| E15 | `to_barycentric` | **Multiple NaNs at once** (2..8 NaN inputs, mixed quiet/signalling, mixed signs) — the payload-selection tie-break | x86 SSE rule: for `op dst, src`, if `dst` is NaN the result is `Q(dst)`, else if `src` is NaN the result is `Q(src)`. The winner therefore depends on the register allocation of the `-O0` build, which the Rust helpers `sub_dst_lhs` / `mul_dst_lhs` / `mul_dst_rhs` / `add_dst_rhs` / `div_dst_lhs` encode | `e15_multi_nan_payload_race` |
| E16 | `to_barycentric` | **`-0.0` inputs / signed-zero results**: `+0.0 + -0.0 = +0.0`, `-0.0 + -0.0 = -0.0`, `x - x = +0.0`, `1.0f/-0.0 = -inf` | sign of zero must match exactly; compared as bits so `+0.0 != -0.0` | `e16_signed_zero` |
| E17 | `to_barycentric` | **Every one of the 2^32 `float` bit patterns is a legal argument** — there is no `enum`, no pointer and no length in the signature, so the classic FFI rejections (`NULL` pointer, zero length, oversized length, out-of-range `enum` value) are **structurally unreachable**. Documented and asserted as such: the tests fuzz fully-random 32-bit patterns in all 8 slots, which is the exact analogue of "a value with no valid variant" for this API | never rejects; always returns two `float`s, bit-identical to Rust | `e17_fully_random_bit_patterns`, `e17_no_pointer_or_enum_params` |

### Rows that are structurally not applicable (recorded for completeness)

The Phase C brief asks for null pointers, zero/oversized lengths and
out-of-range enum values. Those are **not reachable** for this API and the
reason is mechanical, not a judgement call:

```sh
$ grep -cE '\*|\[|enum|size_t|unsigned|int\b' c_src/include/lib.h
0
```

The public header declares exactly one function whose every parameter and whose
return value is `lm_vec2 { float x, y; }`. There is no pointer to be `NULL`, no
count to be `0` or oversized, and no enumerated type to be given an
out-of-range `int`. Row **E17** covers the equivalent adversarial input for a
float-only ABI — the full 32-bit domain of each of the 8 `float` fields,
including all NaN/inf/subnormal encodings, which *is* the "value with no
sensible variant" class for this signature. `e17_no_pointer_or_enum_params` is a
guard test that re-greps the header at test time so this justification cannot
silently rot.

## Which of these "error" behaviours are actually *observable*

Not every internal choice the translation makes is visible through the ABI.
`mutation_check.sh` injects each choice's opposite one at a time and records
whether the suite notices. Five of the twenty mutants provably cannot be
noticed, and the reason is worth writing down because it bounds what Phase C can
possibly verify:

| site | observable? | why |
|------|-------------|-----|
| `lm_sub2` subtraction operand order | **no** (M4) | `SUBSS`/`VSUBSS` already put the minuend in the destination / first source operand, so `sub_dst_lhs(l, r)` and plain `l - r` are the same function on x86-64. The helper only documents intent. |
| denominator `dot00 * dot11` operand order | **no** (M8) | Differs only when `dot00` *and* `dot11` are both NaN. But then the u-numerator begins `mul_dst_lhs(dot11, dot02) = Q(dot11)` and the v-numerator `mul_dst_lhs(dot00, dot12) = Q(dot00)`; a NaN left operand wins the following `subss` and the final `mulss`, so `u = Q(dot11)` and `v = Q(dot00)` no matter what `invDenom` held. |
| denominator `subss` operand order | **no** (M19) | Needs both `dot00*dot11` and `dot01*dot01` to be NaN. A square is NaN only if its operand is, so `dot01` must be NaN — and a NaN `dot01` makes **both** numerators NaN, again masking `invDenom`. |
| `dot01 * dot01` operand order | **no** (M20) | The two operands are the same value. |
| `1.0f / denom` operand order | **no** (M10) | Differs only when both operands are NaN; the dividend is the literal `1.0f`. |
| `lm_dot2` x-term, y-term and `addss` operand order | **yes** (M1, M2, M3, M14) | Caught by `e15_multi_nan_payload_race` / `b16` / `b17`. |
| both numerators' `mulss` and `subss` operand order | **yes** (M9, M15, M16) | Caught. |
| final `× invDenom` operand order | **yes** (M12) | Caught. |
| the SNaN→QNaN quiet bit, and preserving the sign | **yes** (M7, M17) | Caught. |
| `f32` vs `f64` intermediates; FMA contraction | **yes** (M11, M18) | Caught. |

## Provenance of the NaN payloads (important)

The payload behaviour above is a property of the **compiled reference binary**,
not of the C source. Two builds of the identical `c_src/src/lib.c`:

* agree on **200 000 / 200 000** NaN-free inputs (`-O0` vs `-O2`), and
* disagree on **186 698 / 200 000** NaN-carrying inputs.

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no flags, so the reference
`.so` is `-O0`, and the Rust translation reproduces that build exactly —
including on all 293 439 sampled inputs where `-O2` would have answered
differently. Tests `d7`, `d8`, `d9` in `tests/phase_d_build_sensitivity.rs` pin
this down, and `d9` fails with an explanation if the reference build ever gains
optimisation flags.

## Status

All 18 rows (E1–E17 plus E6b) have a passing differential test — see
`tests/phase_c_errors.rs` and the checklist in `VERIFICATION.md`.
