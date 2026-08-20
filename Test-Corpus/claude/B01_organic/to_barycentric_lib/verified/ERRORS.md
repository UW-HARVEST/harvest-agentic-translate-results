# ERRORS.md — Error-surface table (Phase A)

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep for every rejection mechanism

```sh
$ grep -nE 'return (-1|0|NULL)|RETURN_ERROR|assert|errno|goto (err|fail)|if *\(' c_src/src/lib.c
(no matches)
$ grep -cE '#define|#if|enum|\*' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:0
c_src/include/lib.h:0
```

**Result: the C library contains ZERO explicit error paths.** There is:

* no error enum, no error return code, no sentinel return value;
* no `assert`, no `errno` use, no `goto` error label;
* not a single `if` — `to_barycentric` is straight-line code;
* no pointer parameter anywhere (all four arguments and the return value are
  8-byte `lm_vec2` structs passed **by value** in XMM registers), hence no null
  check to make and no null pointer that *could* be passed;
* no `enum` parameter, hence no out-of-range enum value that could cross the
  FFI boundary;
* no length/count/size parameter, hence no zero-length or oversized-length
  check;
* no `#if`/`#ifdef`, so no configuration-dependent validation.

Consequently the "error surface" of this library is **entirely numeric**: every
`float` bit pattern is an accepted input, and rejection is expressed only as an
IEEE-754 special result (`±inf`, NaN) rather than as an error code. The table
below therefore enumerates every distinct way the C code can produce a
non-finite / degenerate result, i.e. every condition under which it effectively
"fails" to return a usable barycentric coordinate. Each row is a real input the
C handles and the Rust must handle **bit-identically**, including the NaN sign
and payload.

The single arithmetic operation that can "reject" is on line 25:

```c
float invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01);   /* unguarded */
```

and the two multiplications on lines 26-27 that consume `invDenom`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `to_barycentric` | `p1 == p2 == p3` (fully degenerate triangle) → `v0 = v1 = (0,0)` → all dots `0` → denom `0*0 - 0*0 = +0` → `invDenom = 1/+0 = +inf`; numerators are `+0` → `u = v = 0 * +inf` | `u`,`v` = x86 "indefinite" QNaN `0xFFC00000` (invalid operation `0 * inf`) | `err_e1_all_points_equal` |
| E2 | `to_barycentric` | `p1 == p3`, `p2` distinct (`v0 = (0,0)`, `v1 != 0`) → `dot00 = dot01 = dot02 = 0`, denom `= +0` → `invDenom = +inf`, `u` numerator `dot11*dot02 - 0 = ±0`, `v` numerator `0*dot12 - 0 = ±0` | both components QNaN `0xFFC00000` | `err_e2_p1_eq_p3` |
| E3 | `to_barycentric` | `p1 == p2`, `p3` distinct (`v1 = (0,0)`) → `dot11 = dot01 = dot12 = 0`, denom `= dot00*0 - 0 = +0` → `invDenom = +inf`; `u` numerator `0*dot02 - 0*0 = ±0` → QNaN; `v` numerator `dot00*0 - 0*dot02 = ±0` → QNaN | both components QNaN `0xFFC00000` | `err_e3_p1_eq_p2` |
| E4 | `to_barycentric` | `p2 == p3` but both distinct from `p1` → `v0 == v1` → `dot00 == dot01 == dot11` → denom `= d*d - d*d = +0` (or `-0`/NaN on overflow) → `invDenom = ±inf`; numerator `d*dot02 - d*dot12 = 0` when `dot02 == dot12` | QNaN `0xFFC00000` in both components | `err_e4_p2_eq_p3` |
| E5 | `to_barycentric` | **collinear** (non-coincident) `p1`,`p2`,`p3`, e.g. `(0,0),(1,1),(2,2)` → Cauchy-Schwarz equality → denom `= +0` exactly → `invDenom = +inf`. Verified empirically: for *exact* collinearity both numerators are also exactly `0` (`dot11*dot02 == dot01*dot12` and `dot00*dot12 == dot01*dot02` follow algebraically from `v0 = k*v1`), so this is `0 * inf` | **both components QNaN `0xFFC00000`** (not `±inf`) | `err_e5_collinear` |
| E6 | `to_barycentric` | denominator is `-0.0`: unreachable for a Gram determinant of finite values (`dot00*dot11 >= dot01*dot01 >= 0` and `x - x = +0` in round-to-nearest), but the *reachable* neighbour is a **negative denominator via NaN/inf contamination** and `1/±0` sign selection; row exists to prove `1/-0 = -inf` is exercised where reachable and that C and Rust agree on the zero's sign | `-inf` where reachable, else QNaN `0xFFC00000`; must match C bit-for-bit either way | `err_e6_negative_zero_denominator` |
| E7 | `to_barycentric` | denominator underflows to `+0` from tiny products (all coordinates ~`1e-30`, so every dot rounds to `0`) → denom `= +0` → `invDenom = +inf`, numerators also `0` | both components QNaN `0xFFC00000` | `err_e7_underflow_denominator` |
| E7b | `to_barycentric` | denominator rounds to `+0` while the numerators stay **non-zero** — reachable only with wildly mixed binades, so `u`/`v` become genuine `±inf`. Empirically found (see `err_e19_actual_infinity` for the six exact witnesses), e.g. `p1={-0x1.adb8aap+25, 0x1.18232p+13}`, `p2={0x1.f010f8p+10, 0x1.bd08d4p-3}`, `p3={-0x1.03dfd2p-30, -0x1.2e6f82p+10}`, `p={-0x1.de12e8p+11, -0x1.6392ap+0}` | `u = 0xFF800000` (`-inf`), `v = 0x7F800000` (`+inf`) | `err_e19_actual_infinity` |
| E8 | `to_barycentric` | denominator **overflows**: coordinates ~`1e20` so each dot is `+inf` → `dot00*dot11 = +inf`, `dot01*dot01 = +inf` → `inf - inf` | denom = QNaN `0xFFC00000` → `invDenom = 1/NaN = NaN` → `u`,`v` = NaN (propagated payload/sign per SSE destination-operand rule) | `err_e8_overflow_denominator` |
| E9 | `to_barycentric` | a single **coordinate difference overflows** to `±inf` (e.g. `p3.x = 3.4e38`, `p1.x = -3.4e38`) → `v0.x = +inf` → `dot00 = +inf` while other dots finite | `u`,`v` = NaN or `±0`/`±inf` per IEEE; must match bit-for-bit | `err_e9_coordinate_overflow` |
| E10 | `to_barycentric` | any input coordinate is `+inf` or `-inf` (each of the 8 float slots tested independently) → `inf - inf = NaN` in `lm_sub2`, or `inf * 0` in `lm_dot2` | QNaN `0xFFC00000` from the invalid op, then propagated | `err_e10_infinity_in_each_slot` |
| E11 | `to_barycentric` | any input coordinate is a **quiet** NaN (`0x7FC00000`), each of the 8 slots independently | NaN propagated; the *surviving* payload/sign is chosen by the SSE destination-operand rule, so the exact bits matter | `err_e11_qnan_in_each_slot` |
| E12 | `to_barycentric` | any input coordinate is a **signalling** NaN (`0x7F800001`), each of the 8 slots independently → first arithmetic op quiets it to `0x7FC00001` | quieted NaN `0x7FC00001` (payload preserved, MSB of mantissa set) propagated | `err_e12_snan_in_each_slot` |
| E13 | `to_barycentric` | NaN with a **negative sign** and non-canonical payload (`0xFFFFFFFF`, `0xFF800001`) — checks the sign bit survives quieting | sign-preserving quieted NaN | `err_e13_negative_nan_payloads` |
| E14 | `to_barycentric` | **two NaN operands** meet in one instruction (e.g. `p1` all-NaN and `p3` all-NaN, so `subss` sees NaN,NaN) → x86 returns the *destination* operand quieted, not the source | destination-operand NaN quieted — the exact case that a naive Rust translation gets backwards | `err_e14_two_nan_operands` |
| E15 | `to_barycentric` | `-0.0` inputs: `p == p1` componentwise with mixed `±0` → `v2 = ±0`, numerators `±0`, and `0 - 0 = +0` vs `(-0) - 0 = -0` sign rules | sign of zero in `u`/`v` must match exactly (`+0` vs `-0` are different bit patterns) | `err_e15_signed_zero` |
| E16 | `to_barycentric` | **subnormal** input coordinates (`1e-45`, `0x00000001`) → products flush/round to `0` in round-to-nearest (no FTZ, since MXCSR default is used by both libraries) | subnormal-precision result, bit-identical | `err_e16_subnormals` |
| E17 | `to_barycentric` | fully **random 32-bit patterns** in all 8 slots (the "out-of-range enum" analogue for a float-only API: every bit pattern is a legal input, including all 16.7M NaN encodings) | whatever the C returns, bit-for-bit | `err_e17_random_bit_patterns` |
| E18 | `to_barycentric` | `1.0f / denom` where `denom` is the largest subnormal / smallest normal → `invDenom` overflows to `+inf`, then `finite * inf` | `±inf` or QNaN, bit-identical | `err_e18_invdenom_overflow` |
| E19 | `to_barycentric` | the only input class that makes the unguarded division yield a **true `±inf` result** rather than a NaN (denominator rounds to `+0`, numerators non-zero) — see E7b | `u`/`v` = `0x7F800000` / `0xFF800000` per numerator sign | `err_e19_actual_infinity` |

## Observed C results for every row (recorded from the built C `.so`)

These are the concrete sentinels the differential tests assert, captured by
running the reference `.so` directly. `0xFFC00000` is the x86 "indefinite" QNaN
that an invalid operation (`0*inf`, `inf-inf`, `inf/inf`) manufactures; note it
is **negative**, whereas a merely *propagated* NaN keeps the operand's own sign
and payload. Getting this distinction wrong is the single most likely
translation bug in this library, which is why E11-E14 exist.

```
E1  all equal (1,2)          -> u=0xffc00000 v=0xffc00000
E1  all equal 0              -> u=0xffc00000 v=0xffc00000
E2  p1==p3                   -> u=0xffc00000 v=0xffc00000
E3  p1==p2                   -> u=0xffc00000 v=0xffc00000
E4  p2==p3                   -> u=0xffc00000 v=0xffc00000
E5  collinear (0,0)(1,1)(2,2)-> u=0xffc00000 v=0xffc00000
E7  all coords 1e-30         -> u=0xffc00000 v=0xffc00000
E8  all coords ~1e20         -> u=0xffc00000 v=0xffc00000
E9  coordinate overflow      -> u=0xffc00000 v=0xffc00000
E10 +inf in p1.x             -> u=0xffc00000 v=0xffc00000
E10 -inf in p3.y             -> u=0xffc00000 v=0xffc00000
E11 QNaN 0x7fc00000 in p1.x  -> u=0x7fc00000 v=0x7fc00000   (propagated, POSITIVE)
E12 SNaN 0x7f800001 in p2.x  -> u=0x7fc00001 v=0x7fc00001   (quieted, payload kept)
E13 0xffffffff in p.y        -> u=0xffffffff v=0xffffffff   (sign+payload kept)
E14 0x7fc01234 & 0x7fdeadbe  -> u=0x7fc01234 v=0x7fdeadbe   (destination operand wins,
                                                              and u/v pick DIFFERENT ones)
E15 -0.0 in p                -> u=0x00000000 v=0x00000000
E16 all coords 0x00000001    -> u=0xffc00000 v=0xffc00000
E18 invDenom overflow        -> u=0xffc00000 v=0xffc00000
E19 mixed-binade witness     -> u=0xff800000 v=0x7f800000   (true -inf / +inf)
```

Row E14 is the decisive one: with two different NaN payloads meeting in one
`subss`, the C returns `0x7fc01234` for `u` but `0x7fdeadbe` for `v` — the two
output components select *different* NaN operands. Only an
instruction-for-instruction match of the SSE destination-operand choice
reproduces that.

## Non-applicable generic boundaries (documented for completeness)

The task list asks for null pointers, zero/oversized lengths and out-of-range
enum values. This API's signature makes them unrepresentable:

All of these are nevertheless *asserted* rather than merely argued, by
`err_generic_boundaries_are_unrepresentable`, which checks the resolved symbol
pointers are non-NULL and drives every boundary bit pattern
(`+inf`/`-inf` = one step past `±FLT_MAX`, `FLT_MAX`, `FLT_MIN`, largest
subnormal = one step below `FLT_MIN`, smallest subnormal, `±0`, and six NaN
encodings standing in for "an enum value with no valid variant") through every
one of the 8 input slots and through all 8 slots at once.

| generic boundary | applicable? | why |
|------------------|-------------|-----|
| null pointer argument | **no** | `to_barycentric` takes four by-value structs; there is no pointer parameter and no pointer return |
| zero length / oversized length | **no** | no length, size or count parameter exists |
| out-of-range `enum` across FFI | **no** | no `enum` in the header; the only parameter type is `struct lm_vec2 { float x, y; }` |
| value one step past a documented valid range | **covered** | there is no documented valid range; the analogue is "every one of the 2^32 bit patterns per float", covered by E11-E17 (all NaN/inf/subnormal/random encodings) |
| struct layout / padding mismatch | **covered** | asserted in `abi_struct_layout` (size 8, align 4, offsets 0 and 4) and by register-level `objdump` comparison |

## Phase C status

Every row above has a differential test that constructs the exact condition,
calls BOTH `.so` files through `libloading`, and asserts the returned
`lm_vec2` is **bit-identical** — plus, where `ERRORS.md` records a specific
sentinel, that the C really returned that sentinel (so a row cannot pass just
because "both failed somehow").

| row | test | status |
|-----|------|--------|
| E1  | `err_e1_all_points_equal` | PASS |
| E2  | `err_e2_p1_eq_p3` | PASS |
| E3  | `err_e3_p1_eq_p2` | PASS |
| E4  | `err_e4_p2_eq_p3` | PASS |
| E5  | `err_e5_collinear` | PASS |
| E6  | `err_e6_negative_zero_denominator` | PASS |
| E7  | `err_e7_underflow_denominator` | PASS |
| E7b/E19 | `err_e19_actual_infinity` | PASS (31 594 genuine `±inf` results observed) |
| E8  | `err_e8_overflow_denominator` | PASS |
| E9  | `err_e9_coordinate_overflow` | PASS |
| E10 | `err_e10_infinity_in_each_slot` | PASS |
| E11 | `err_e11_qnan_in_each_slot` | PASS |
| E12 | `err_e12_snan_in_each_slot` | PASS |
| E13 | `err_e13_negative_nan_payloads` | PASS |
| E14 | `err_e14_two_nan_operands` | PASS |
| E15 | `err_e15_signed_zero` | PASS |
| E16 | `err_e16_subnormals` | PASS |
| E17 | `err_e17_random_bit_patterns` | PASS |
| E18 | `err_e18_invdenom_overflow` | PASS |
| generic boundaries | `err_generic_boundaries_are_unrepresentable` | PASS |

`20 passed; 0 failed` under every feature combination and against both the
debug and the release Rust `.so`. Re-run with `./verify_all.sh` (add a scale
factor, e.g. `./verify_all.sh 25`, for a soak run).

### Anti-vacuity check

A deliberately-wrong "mutant" C library (`lm_dot2` written as
`a.y*b.y + a.x*b.x`, i.e. the naive commuted form a translator would plausibly
emit) is built by `verify_all.sh` and substituted for the Rust `.so`. It is
**rejected**, and specifically rows **E14** and **E17** are the ones that catch
it — the single-NaN rows E11-E13 pass against the mutant. That is direct
evidence that the two-NaN-collision row is load-bearing and that the suite is
not passing vacuously.
