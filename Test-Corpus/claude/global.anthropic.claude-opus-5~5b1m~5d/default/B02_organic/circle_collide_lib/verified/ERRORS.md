# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Greps run:

```sh
grep -n 'return' c_src/src/lib.c            # 17 return statements, all listed below
grep -n 'assert\|RETURN_ERROR\|errno\|NULL\|abort\|exit' c_src/src/lib.c   # -> no matches
grep -n 'if\|switch\|case\|default\|?' c_src/src/lib.c
grep -nE '#define|[A-Z_]+_(MIN|MAX)|INT_|FLT_' c_src/src/lib.c            # -> no matches
```

**Finding:** this library has NO error-reporting channel. There are no
`assert`s, no error enums, no sentinel error returns, no `errno` use, no
explicit null checks, and no named min/max constants. Every function returns a
value computed unconditionally from its arguments. Therefore the "rejection"
surface consists of exactly one explicit rejection branch (the `switch`
`default:`), plus the value-domain boundaries every float/pointer C API has,
which are enumerated below because the completion gate requires them even when
absent from the source's own error handling.

Legend: "expected C result" is what the C `.so` actually does, and is what the
Rust `.so` must reproduce bit-for-bit.

## Explicit rejection branches in the C source

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `c2Collided` (`lib.c:112-113`, `default:`) | `typeB` is any `int` that is not `0`/`1`/`2` — i.e. out-of-range enum value crossing FFI. Tested values: `3`, `4`, `-1`, `-2`, `100`, `0x7fffffff` (`INT_MAX`), `-0x80000000` (`INT_MIN`), `0x100` , `0xffff`. Note a value like `0x100000000i64` truncated to `int` becomes `0` and is *valid*. | returns `0`; `A`/`B` are never dereferenced, so even null/garbage pointers are safe in this branch |

That is the only rejection the code performs. Rows 2+ below are the generic
C-API boundary conditions mandated by the completion gate.

## Generic boundary / degenerate-input conditions

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 2 | `c2Collided` | `A == NULL`, `B == NULL`, with `typeB` out of range (`3`, `-1`, `INT_MIN`) | returns `0` without dereferencing (see #1). Both `.so`s must agree. |
| 3 | `c2Collided` | `typeB` valid (`0`/`1`/`2`) but `B` points at a buffer *shorter* than the selected shape, placed at the end of a page so the shape's tail is unmapped | genuine UB in C (out-of-bounds read). **Not tested** — cannot be observed portably; documented for completeness. Instead tested: `B` pointing at a buffer of the *correct* size but at an unaligned (odd) address, which is well-defined in practice for both and must agree. |
| 4 | `c2Collided` | `typeB` valid, `A`/`B` aliasing the *same* object (e.g. `c2Collided(p, p, 0)`) | no aliasing assumption in C (args are `const void*`), computes normally. With a real `c2Circle` in the buffer and `typeB=CIRCLE`: distance is `0` and `r2 == (2r)^2`, so the strict `<` returns `1` for any nonzero non-NaN `r` and `0` for `r == ±0` or NaN. With a `c2Capsule` in the buffer the blind `*(c2Circle *)A` cast makes `A.r` the capsule's `b.x` (offset 8), not its `r` — that reinterpretation is part of the contract and is preserved |
| 5 | `c2CircletoCapsule` (`lib.c:93`) | unguarded division `da / c2Dot(n,n)` with a zero divisor. Two sub-cases, both tested: **(a)** exactly degenerate capsule `B.a == B.b`, so `n == (0,0)` and `c2Dot(n,n) == 0` — here `da` and `db` are both `±0`, and `±0 < 0` is false, so the `else` branch runs and the division is **not** reached; **(b)** *underflowing* capsule where `n`'s components are ~`1e-23` so `c2Dot(n,n)` rounds to `0` while `da >= 0` and `db < 0` — branch B runs and the division really is `x / 0`. Sub-case (b) is **reachable** (measured: hit 10× in row 40 of `CONFIGS.md` and 20× in `err05`, asserted `> 0`), producing `±inf` or `NaN`, then `c2Mulvs` yields `NaN`, `d2 = NaN`. **(c)** also tested: `c2Dot(n,n) == +inf` on branch B, giving a `0` or `NaN` quotient. | returns `0` in every zero-divisor case (`NaN < r*r` is false). All three sub-cases must match the C `.so` exactly. |
| 6 | `c2CircletoCapsule` | `A.p` NaN in x and/or y, any capsule | every comparison against NaN is false: `da < 0` false → `db < 0` false → `else` branch → `d2` NaN → `d2 < r*r` false → returns `0` |
| 7 | `c2CircletoCapsule` | `A.r` and/or `B.r` NaN, or `A.r + B.r` NaN (`+inf + -inf`) | `r*r` is NaN → comparison false → returns `0` |
| 8 | `c2CircletoCapsule` | `A.r`/`B.r` huge so `r = A.r+B.r` overflows to `+inf` | `d2 < inf` is `1` for any finite `d2`; `inf < inf` is `0` when `d2` is also `inf` |
| 9 | `c2CircletoCapsule` | `A.r` or `B.r` negative (no validity check in C) | computed as-is; `r*r >= 0` so a negative radius behaves like its absolute value |
| 10 | `c2CircletoCircle` (`lib.c:70-72`) | `A.r + B.r` overflows to `+inf`, or is NaN, or radii negative | `d2 < r2` with `r2 = inf` → `1` for finite `d2`; NaN → `0`; negative radii squared → positive |
| 11 | `c2CircletoCircle` | `B.p - A.p` overflows to `±inf` (e.g. `±3.4e38`), so `d2 = inf` | `inf < r2` is `0` unless `r2` is `inf`, then also `0` |
| 12 | `c2CircletoCircle` | either centre coordinate NaN → `d2` NaN | `NaN < r2` false → returns `0` |
| 13 | `c2CircletoAABB` (`lib.c:76`) | **inverted AABB**, `B.min > B.max` componentwise (no validity check). `c2Clampv` = `c2Maxv(lo, c2Minv(a, hi))` so the result collapses to `lo` whenever `lo > hi` | computed as-is, no error; must match bit-for-bit |
| 14 | `c2CircletoAABB` | `B.min`/`B.max` NaN. `c2Minv`/`c2Maxv` use raw `<`/`>` ternaries, so a NaN comparison is false and the *second* operand is selected: `c2Minv(a,hi)` → `hi`, `c2Maxv(lo,·)` → the min result | no error; NaN propagates into `d2` → returns `0` (or `1` if NaN is cancelled) — must match exactly |
| 15 | `c2CircletoAABB` | `A.r` NaN or `A.r*A.r` overflow to `inf`, `A.r` negative | `d2 < r2`; NaN → `0`; `inf` → `1` for finite `d2`; negative `A.r` behaves as `|A.r|` |
| 16 | `c2CircletoAABB` | `A.p` NaN | `L` = clamp of NaN selects `B.max` then `c2Maxv(min, max)`; `ab = NaN - x = NaN` → `d2` NaN → returns `0` |
| 17 | `c2Dot` (`lib.c:64`) | operands that make `a.x*b.x` and `a.y*b.y` `+inf` and `-inf` → `inf + -inf = NaN` | returns NaN; the exact NaN bit pattern (default qNaN `0x7fc00000`) must match |
| 18 | `c2Dot` | both multiplicands NaN with *different* payloads/sign bits — SSE `mulss`/`addss` return the *destination* operand's NaN, so the result depends on the instruction operand order the compiler chose | must match the C `.so` **bit-for-bit**; this is why `src/lib.rs` pins the order with `mulss`/`addss` inline asm instead of plain `*`/`+` |
| 19 | `c2Mulvs` (`lib.c:37-41`) | `b` NaN and `a.x`/`a.y` NaN with different payloads; also `0 * inf` → NaN, `inf * 0` | GCC does NOT vectorise at `-O0`: it emits `movss xmm0,[a.x]; mulss xmm0,[b]` per lane, so the *vector lane* is the destination and `a`'s NaN wins over `b`'s. Result bits must match. |
| 20 | `c2Maxv` / `c2Minv` (`lib.c:43-51`) | either operand NaN → ternary comparison false → returns the `b` operand (NOT IEEE `maxNum`/`minNum`, NOT `f32::max`) | must match; `f32::max`/`f32::min` would diverge here |
| 21 | `c2Maxv` / `c2Minv` | `-0.0` vs `+0.0` (comparison is equal, so the ternary is false and `b` is returned regardless of sign) | `c2Maxv((+0,·),(-0,·))` returns `-0.0`; sign bit must match |
| 22 | `c2Clampv` (`lib.c:53-55`) | `lo > hi` (inverted range), and NaN in `a`, `lo`, or `hi` | no validation; composed ternary behaviour, must match bit-for-bit |
| 23 | `c2Sub` (`lib.c:57-61`) | `inf - inf` → NaN; `±inf - finite`; overflow to `inf`; subnormal cancellation to `±0` | returns the IEEE result; sign of zero (`x - x = +0`, `-0 - +0 = -0`) must match |
| 24 | `c2V` (`lib.c:30-35`) | any bit pattern including signalling NaN (`0x7f800001`), `-NaN`, subnormals, `±0` | pure struct construction — must round-trip the exact 32-bit patterns through the XMM-packed return with no canonicalisation (a signalling NaN must NOT be quieted) |
| 25 | `circle_collide` (`lib.h:1`, `lib.c:117`) | `x`/`y`/`r` = NaN | all three sub-tests compare against NaN and fail → returns `0` |
| 26 | `circle_collide` | `x`/`y`/`r` = `±inf` | `d2` becomes `inf`/NaN; per rows 10-16 → returns `0` |
| 27 | `circle_collide` | `r` negative (e.g. `-20.0`), `r == 0`, `r == -0.0` | no validation in C; `r*r`/`(r+R)^2` used, so `r = -20` collides like `r = +20` for the AABB test but `A.r+B.r` changes sign behaviour for circle/capsule; must match |
| 28 | `circle_collide` | `r` huge (`3.4e38`, `FLT_MAX`) so `r + 20.0f` overflows to `inf` → `r2 = inf` | returns bit 0 set (`inf` beats any finite `d2`); exact packed result must match |

---

## Row → test mapping and status

Every row has a differential test in `tests/phase_c_errors.rs` that constructs
the exact condition, calls BOTH `.so`s, and asserts they return the SAME value.
Where the C source pins a specific sentinel (`0` from `default:`, `0` from a
`NaN <` comparison, `1` from `finite < inf`), the test asserts that sentinel too,
so a test that silently stopped exercising its branch would fail.

Status recorded after `./verify.sh`: **28 / 28 passing**, against both the debug
and the release Rust `.so`.

| # | test fn (`tests/phase_c_errors.rs`) | [x] |
|---|-------------------------------------|-----|
| 1 | `err01_collided_out_of_range_enum` — 19 invalid `int` values × 200 random payloads, plus the `-4..=6` boundary sweep | [x] |
| 2 | `err02_collided_null_pointers_bad_type` | [x] |
| 3 | `err03_collided_unaligned_shapes` — 9×9 odd offsets × 3 types | [x] |
| 4 | `err04_collided_aliasing` | [x] |
| 5 | `err05_capsule_division_by_zero` — sub-cases (a) degenerate, (b) underflow→`x/0` (asserted reachable), (c) `inf` divisor | [x] |
| 6 | `err06_capsule_nan_centre` | [x] |
| 7 | `err07_capsule_nan_radius` | [x] |
| 8 | `err08_capsule_radius_overflow` | [x] |
| 9 | `err09_capsule_negative_radii` | [x] |
| 10 | `err10_circle_radius_overflow_nan_negative` | [x] |
| 11 | `err11_circle_centre_overflow` | [x] |
| 12 | `err12_circle_nan_centre` | [x] |
| 13 | `err13_aabb_inverted_box` | [x] |
| 14 | `err14_aabb_nan_bounds` | [x] |
| 15 | `err15_aabb_radius_specials` | [x] |
| 16 | `err16_aabb_nan_centre` | [x] |
| 17 | `err17_dot_inf_minus_inf` | [x] |
| 18 | `err18_dot_nan_operand_order` — 13^4 exhaustive | [x] |
| 19 | `err19_mulvs_nan_and_zero_times_inf` — 14^3 exhaustive | [x] |
| 20 | `err20_minmax_nan_returns_b` — 14^4 × both fns | [x] |
| 21 | `err21_minmax_signed_zero` | [x] |
| 22 | `err22_clampv_inverted_and_nan` | [x] |
| 23 | `err23_sub_inf_and_signed_zero` — 12^4 exhaustive | [x] |
| 24 | `err24_c2v_no_canonicalisation` — 144 bit patterns squared, sNaN must stay signalling | [x] |
| 25 | `err25_circle_collide_nan` | [x] |
| 26 | `err26_circle_collide_infinities` | [x] |
| 27 | `err27_circle_collide_zero_and_negative_r` | [x] |
| 28 | `err28_circle_collide_huge_r` | [x] |

## Bugs this table actually caught

The error surface here is float-semantics-shaped rather than error-code-shaped,
and the NaN rows found **four real divergences** in the Rust translation, all in
SSE operand order (the `mulss`/`addss` destination operand is the one whose NaN
payload propagates, and GCC's `-O0` choice is not the source-text order):

| function | C (`objdump -d`) | Rust before | fixed to |
|----------|------------------|-------------|----------|
| `c2Mulvs` | `movss xmm0,[a.x]; mulss xmm0,[b]` → dst = `a` lane | `mul_keep_lhs_nan(b, a.x)` | `mul_keep_lhs_nan(a.x, b)` |
| `c2Dot` y lane | `movss xmm2,[a.y]; movss xmm0,[b.y]; mulss xmm0,xmm2` → dst = `b.y` | `mul_keep_lhs_nan(a.y, b.y)` | `mul_keep_lhs_nan(b.y, a.y)` |
| `c2Dot` sum | `addss xmm0,xmm1` with `xmm0` = the *y* product | `add_keep_lhs_nan(px, py)` | `add_keep_lhs_nan(py, px)` |
| `c2CircletoCircle`, `c2CircletoCapsule` | `movss xmm1,[A.r]; movss xmm0,[B.r]; addss xmm0,xmm1` → dst = `B.r` | `A.r + B.r` (commutable by LLVM) | `add_keep_lhs_nan(B.r, A.r)` |

Note the C `.so` produced by `c_src/CMakeLists.txt` has no `CMAKE_BUILD_TYPE`,
so it is compiled at `-O0` and calls `c2Sub`/`c2Dot`/`c2Mulvs`/`c2Clampv`
through the PLT rather than inlining them. The operand orders above are read
directly from that object.
