# ERRORS.md — error / rejection surface (Phase C)

Mechanically derived from `c_src/src/lib.c`. The library has **no error enum, no
`errno`, no `assert`, no null checks and no allocation**; every rejection is a
`return 0` (or a fall-off-the-end in `c2CastRay`).  Rows were produced by
grepping every `return 0`, `return 1`, `!`, `<`, `>`, `==` guard in the source:

```
$ grep -n "return 0;\|return 1;\|return !\|return c2\|return d2\|switch" c_src/src/lib.c
```

Each row has a differential test that constructs exactly that condition and
asserts the C and Rust `.so`s return the SAME value (and leave `out` in the same
state, bit-for-bit).  Legend: **[x]** = passing.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| E01 | `c2RaytoCircle` (`lib.c:100`) | `disc = b*b - c < 0` — the infinite ray line misses the circle | returns `0`, `*out` untouched | [x] |
| E02 | `c2RaytoCircle` (`lib.c:100`) | `disc` is NaN (e.g. `inf - inf` from `inf` fields) — `disc < 0` is **false**, so the NaN flows on and `t >= 0` then fails | returns `0`, `*out` untouched | [x] |
| E03 | `c2RaytoCircle` (`lib.c:103`) | `t = -b - sqrt(disc) < 0` — impact behind the ray origin (origin inside, or shape behind) | returns `0`, `*out` untouched | [x] |
| E04 | `c2RaytoCircle` (`lib.c:103`) | `t > A.t` — impact beyond the ray's length (incl. `A.t == 0` and `A.t < 0`) | returns `0`, `*out` untouched | [x] |
| E05 | `c2RaytoCircle` (`lib.c:103`) | `A.t` is NaN ⇒ `t <= A.t` is false | returns `0`, `*out` untouched | [x] |
| E06 | `c2AABBtoAABB` (`lib.c:113`) | `d0`: `B.max.x < A.min.x` (B entirely left of A) | returns `0` | [x] |
| E07 | `c2AABBtoAABB` (`lib.c:114`) | `d1`: `A.max.x < B.min.x` | returns `0` | [x] |
| E08 | `c2AABBtoAABB` (`lib.c:115`) | `d2`: `B.max.y < A.min.y` | returns `0` | [x] |
| E09 | `c2AABBtoAABB` (`lib.c:116`) | `d3`: `A.max.y < B.min.y` | returns `0` | [x] |
| E10 | `c2AABBtoAABB` (`lib.c:117`) | any coordinate NaN ⇒ every `<` is false ⇒ `!(0)` | returns `1` (accepts!) | [x] |
| E11 | `c2RaytoAABB` (`lib.c:145`) | the ray's own bounding box does not overlap `B` | returns `0`, `*out` untouched | [x] |
| E12 | `c2RaytoAABB` (`lib.c:156`) | `d > 0` — separating axis along the ray normal `c2Skew(ab)` | returns `0`, `*out` untouched | [x] |
| E13 | `c2RaytoAABB` (`lib.c:174`) | `hit == 0` — all four `t_i > 1` | returns `0`, `*out` untouched | [x] |
| E14 | `c2RayToPlane_OneDimensional` (`lib.c:126`) | `da < 0` (ray origin on the far side of that plane) ⇒ that `t_i` is `0` | contributes `t_i = 0` (observable through `out->t`/`out->n`) | [x] |
| E15 | `c2RayToPlane_OneDimensional` (`lib.c:135`) | `da - db == 0` (ray parallel to that plane, e.g. `A.t == 0`) ⇒ `t_i = 0` instead of a division | contributes `t_i = 0` (no `0/0` NaN) | [x] |
| E16 | `c2AABBtoPoint` (`lib.c:218-221`) | each of `d0`: `B.x < A.min.x`, `d1`: `B.y < A.min.y`, `d2`: `B.x > A.max.x`, `d3`: `B.y > A.max.y` (4 separate rejections) | returns `0` | [x] |
| E17 | `c2CircleToPoint` (`lib.c:228`) | `d2 >= A.r * A.r` — point outside or exactly on the circumference | returns `0` | [x] |
| E18 | `c2CircleToPoint` (`lib.c:228`) | `A.r == 0` ⇒ `d2 < 0` impossible | returns `0` for every point | [x] |
| E19 | `c2CircleToPoint` (`lib.c:228`) | `A.r < 0` ⇒ `r*r > 0`, so a *negative* radius still accepts points (C quirk, must be reproduced) | returns `1` inside `\|r\|` | [x] |
| E20 | `c2RaytoCapsule` (`lib.c:260-264`, `291`) | big condition false: `yAe.x*yAp.x >= 0` **and** `min(\|yAe.x\|,\|yAp.x\|) >= B.r` | returns `0`, but `out->n = c2Norm(b-a)` and `out->t = 0` **have already been written** | [x] |
| E21 | `c2RaytoCapsule` (`lib.c:272-283`) | delegated `c2RaytoCircle` misses (E01/E03/E04 inside the end-cap) | returns `0` with `out` left holding the pre-written `{0, norm(b-a)}` | [x] |
| E22 | `c2RaytoCapsule` (`lib.c:233`) | degenerate capsule `B.a == B.b` ⇒ `c2Norm((0,0))` = `(NaN,NaN)` ⇒ every derived value NaN | no rejection: NaN propagates, `c2AABBtoPoint` accepts ⇒ returns `1` | [x] |
| E23 | `c2CastRay` (`lib.c:295`) | `typeB` outside `{0,1,2}` (`3`, `-1`, `INT_MIN`, `INT_MAX`, random) — the `switch` has **no `default:`** and no trailing `return`, so control falls off the end of a non-`void` function: **undefined behaviour** | `*out` untouched, no crash; the return value is whatever the caller left in `%rax` (the `-O0` artifact never writes `%rax` on this path) — unspecified, so the test asserts the *observable, defined* part (`out` untouched, both libraries survive) | [x] |
| E24 | `c2CastRay` (`lib.c:302`) | the `return 0;` after the `C2_TYPE_CAPSULE` case is dead code (the preceding `return` always fires) | unreachable — no input reaches it | n/a |
| E25 | `c2CastRay` / `c2Rayto*` | `out == NULL` on a **miss** path (circle/AABB never write on miss) | returns `0`, no fault | [x] |
| E26 | `c2RaytoCapsule` | `out == NULL` (capsule writes `out->n`/`out->t` *before* any test) | SIGSEGV — asserted identical for both `.so`s in a forked child | [x] |
| E27 | `c2RaytoCircle`, `c2RaytoAABB` | `out == NULL` on a **hit** path | SIGSEGV — asserted identical for both `.so`s in a forked child | [x] |
| E28 | `c2CastRay` | `B == NULL` with a valid `typeB` (dereferenced by `*(c2Circle*)B`) | SIGSEGV — asserted identical for both `.so`s in a forked child | [x] |
| E29 | `gen_ray` | `cast1`/`cast2`/`cast3 == NULL`: `cast2` (capsule) always faults; `cast1`/`cast3` fault only when that shape is hit | SIGSEGV — asserted identical for both `.so`s in a forked child | [x] |
| E30 | `c2Div`, `c2Norm` | `b == 0`/`±0` ⇒ `1.0f/0` = `±inf` then `inf*0 = NaN`; `c2Norm((0,0))` ⇒ `(NaN,NaN)` | no trap, `±inf`/NaN propagate bit-exactly | [x] |
| E31 | `c2Len`, `c2RaytoCircle` | `sqrtf` of a NaN argument (`disc` NaN survives the `disc < 0` guard) | NaN with the input's sign/payload, quieted | [x] |
| E32 | `c2Mulvs`, `c2Dot`, `c2MulmvT`, … | `inf * 0`, `inf - inf` ⇒ the x86 "indefinite" QNaN `0xffc00000` | that exact bit pattern (not `0x7fc00000`) | [x] |
| E33 | `c2RaytoCircle` / `c2RaytoCapsule` | negative radius (`B.r < 0`) — `r*r` is still positive, `c2V(-B.r, 0)` inverts `capsule_bb` | no rejection; must match bit-for-bit | [x] |
| E34 | `gen_ray` | out-of-range/degenerate geometry cannot produce a value outside `0..=7`; every combination of the three casts | `0 <= ret <= 7`, identical bits in all three `c2Raycast`s | [x] |
| E35 | `c2CastRay` | enum value `2` (`C2_TYPE_CAPSULE`) is matched by the `switch`'s first `je` at `-O0`; values `0`,`1` by the following compares; the `ja` treats `typeB` as **unsigned**, so `-1` is rejected like `3` | identical dispatch for both libraries | [x] |
| E36 | `c2Rayto*` (all three) | a **misaligned** `c2Raycast *out` (odd address, offsets 1..7) — x86 `movss`/`movq` have no alignment requirement, so the C just writes | writes the same 12 bytes at the odd address, returns normally | [x] |

## Notes on the two non-testable rows

* **E24** is dead code: the `return 0;` after `case C2_TYPE_CAPSULE:`'s
  `return` statement can never execute, so there is no input to test it with.
* **E23**'s *return value* is the one thing in this library that no
  implementation can reproduce, because it is genuinely undefined:

  | build of `c_src/src/lib.c` | value returned for an out-of-range `typeB` |
  |---|---|
  | `-O0` (the CMake default, i.e. the reference artifact) | whatever the **caller** left in `%rax` — the function never writes `%rax` on that path |
  | `-O2` / `-O3` | the low 32 bits of the `B` pointer (`mov %rdi,%rax` in the prologue) |

  The Rust translation reproduces the `-O2`/`-O3` artifact (`B`'s low 32 bits)
  and documents this in `src/lib.rs`; the differential test asserts the parts
  that ARE defined — that `*out` is untouched and that neither library faults —
  for `-1`, `3`, `4`, `5`, `7`, `99`, `1000`, `INT_MIN`, `INT_MAX` and 2 000
  random out-of-range `int`s.

## Cross-check: how the rows were derived

```
$ grep -nE "return 0;|return 1;|return !|return c2|return d2|switch|if \(" c_src/src/lib.c | wc -l
```

Every `if`/`switch` guard in `lib.c` maps to at least one row above:
`c2RaytoCircle` 3 guards (E01-E05), `c2AABBtoAABB` 4 (E06-E10),
`c2RaytoAABB` 3 (E11-E13), `c2RayToPlane_OneDimensional` 3 (E14, E15),
`c2AABBtoPoint` 4 (E16), `c2CircleToPoint` 1 (E17-E19),
`c2RaytoCapsule` 8 (E20-E22, plus B42-B51 in `CONFIGS.md`),
`c2CastRay` 1 `switch` (E23, E24, E35), `gen_ray` 0 (E34).

## Row → test-function traceability

Verified mechanically by `./audit_rows.py`.

| row | test function | file |
|-----|---------------|------|
| E01 | `b23_e01_miss` | `tests/t3_ray_circle.rs` |
| E02 | `b30_e02_e05_special_in_each_field` | `tests/t3_ray_circle.rs` |
| E03 | `b22_e03_origin_inside` | `tests/t3_ray_circle.rs` |
| E04 | `b21_e04_t_equals_ray_length`, `b25_b26_ray_length_zero_negative` | `tests/t3_ray_circle.rs` |
| E05 | `b30_e02_e05_special_in_each_field` | `tests/t3_ray_circle.rs` |
| E06 | `b13_e06_e09_aabb_separated_each_axis` (axis 0) | `tests/t2_overlap.rs` |
| E07 | `b13_e06_e09_aabb_separated_each_axis` (axis 1) | `tests/t2_overlap.rs` |
| E08 | `b13_e06_e09_aabb_separated_each_axis` (axis 2) | `tests/t2_overlap.rs` |
| E09 | `b13_e06_e09_aabb_separated_each_axis` (axis 3) | `tests/t2_overlap.rs` |
| E10 | `b16_e10_aabb_nan_inf` | `tests/t2_overlap.rs` |
| E11 | `b40_e11_e13_specials_per_field` | `tests/t4_ray_aabb.rs` |
| E12 | `b34_e12_separating_axis_reject` | `tests/t4_ray_aabb.rs` |
| E13 | `b40_e11_e13_specials_per_field` (all-NaN geometry ⇒ `hit == 0`) | `tests/t4_ray_aabb.rs` |
| E14 | `b33_e14_ray_inside_box`, `b32_b39_grid_sweep` | `tests/t4_ray_aabb.rs` |
| E15 | `b36_e15_zero_length_ray` | `tests/t4_ray_aabb.rs` |
| E16 | `b17_e16_aabb_to_point` (all 4 axes + edges/corners) | `tests/t2_overlap.rs` |
| E17 | `b18_e17_e19_circle_to_point` | `tests/t2_overlap.rs` |
| E18 | `b18_e17_e19_circle_to_point` (`r == 0`) | `tests/t2_overlap.rs` |
| E19 | `b18_e17_e19_circle_to_point` (`r < 0`) | `tests/t2_overlap.rs` |
| E20 | `b42_b51_e20_e21_all_branches` (`Branch::NoHit`, 72 059 samples) | `tests/t5_ray_capsule.rs` |
| E21 | `b42_b51_e20_e21_all_branches` (delegated-miss branches) | `tests/t5_ray_capsule.rs` |
| E22 | `b52_e22_degenerate_capsule` | `tests/t5_ray_capsule.rs` |
| E23 | `e23_e35_out_of_range_enum` | `tests/t6_castray.rs` |
| E24 | *(dead code — unreachable, see note above)* | — |
| E25 | `e25_null_out_on_miss` | `tests/t8_null_and_crash.rs` |
| E26 | `e26_e29_null_deref_crash_parity` (`capsule_null_out`) | `tests/t8_null_and_crash.rs` |
| E27 | `e26_e29_null_deref_crash_parity` (`circle_null_out_hit`, `aabb_null_out_hit`) | `tests/t8_null_and_crash.rs` |
| E28 | `e26_e29_null_deref_crash_parity` (`castray_null_b_0/1/2`) | `tests/t8_null_and_crash.rs` |
| E29 | `e26_e29_null_deref_crash_parity` (`gen_ray_null_cast2`, `gen_ray_null_all`) + `e25_null_out_on_miss` | `tests/t8_null_and_crash.rs` |
| E30 | `b29_e30_degenerate_direction`, `e32_indefinite_nan_bits` | `tests/t3_ray_circle.rs`, `tests/t9_misc_errors.rs` |
| E31 | `e31_sqrtf_nan_parity` | `tests/t9_misc_errors.rs` |
| E32 | `e32_indefinite_nan_bits` | `tests/t9_misc_errors.rs` |
| E33 | `b27_e33_radius_variants`, `b54_e33_radius_variants` | `tests/t3_ray_circle.rs`, `tests/t5_ray_capsule.rs` |
| E34 | `b62_e34_all_hit_masks` | `tests/t7_gen_ray.rs` |
| E35 | `e23_e35_out_of_range_enum` | `tests/t6_castray.rs` |
| E36 | `e36_misaligned_out_pointer` | `tests/t9_misc_errors.rs` |
