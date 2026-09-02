# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `switch` with no
`default:`, every `default:` arm, every `return 0` / `return 1` guard, every
NULL check (`if (!ptr)`, `if (ptr)`), every comparison against a limit constant
(`FLT_MAX`, `FLT_EPSILON`, `-1.0e8f`, `iter < 20`, `2.0f`), and every division
that can divide by zero. The library has **no** error enum, no `assert`, no
`RETURN_ERROR` macro and no `errno` use: failures are expressed as sentinel
return values (`0`, `(0,0)`, `0.0f`) or as "leave the output untouched".

Legend for the *expected C result* column: values were read off the C source and
then confirmed against the compiled C `.so` by the differential tests in
`tests/errors.rs`.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `c2Collided` | `typeA` not in {0,1,2} (outer `switch` `default:`) | returns `0`, neither shape dereferenced |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` not in {0,1,2} (`default:`) | returns `0` |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` not in {0,1,2} (`default:`) | returns `0` |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` not in {0,1,2} (`default:`) | returns `0` |
| 5 | `c2Collided` | `A`/`B` NULL but type valid — C dereferences unconditionally | segfault (UB); NOT exercised, documented only |
| 6 | `omni_collide` | `type_a` not in {0,1,2} | returns `0` (reaches `c2Collided` outer `default:`) |
| 7 | `omni_collide` | `type_b` not in {0,1,2}, `type_a` valid | returns `0` (inner `default:`) |
| 8 | `omni_collide` | both types out of range (incl. negative, `INT_MIN`, `INT_MAX`) | returns `0` |
| 9 | `ptr_from_parts` | `typ` not in {0,1,2}: `switch` has no `default:` and control falls off the end of a non-`void` function | **UB** — indeterminate return value. Rust returns NULL. Observationally equivalent because every caller (`omni_collide`→`c2Collided`) hits a `default: return 0` before dereferencing. Compared only for the three valid `typ` values; row 6/7/8 cover the invalid ones end-to-end. |
| 10 | `c2MakeProxy` | `type` not in {0,1,2}: `switch` has no `default:` | `*p` left completely untouched (caller sees uninitialised/prior contents). Tested by pre-filling `*p` with a known pattern and asserting both C and Rust leave all 72 bytes unchanged. |
| 11 | `c2GJK` | `typeA`/`typeB` out of range → proxy never filled | C reads an uninitialised stack `c2Proxy`, hands the garbage `count` to `c2Support` and walks off `verts[8]`: **verified to SIGSEGV**. Unexercisable UB; `tests/errors.rs::row11_gjk_bad_enum_documented_ub` asserts the reachable half instead (`c2MakeProxy` leaves the proxy byte-identical, and rows 1-8 show the public entry points reject the type before `c2GJK` is reached). |
| 12 | `c2GJK` | `ax_ptr == NULL` | substitutes `c2xIdentity()` |
| 13 | `c2GJK` | `bx_ptr == NULL` | substitutes `c2xIdentity()` |
| 14 | `c2GJK` | `outA == NULL` | no write performed, no crash |
| 15 | `c2GJK` | `outB == NULL` | no write performed, no crash |
| 16 | `c2GJK` | `iterations == NULL` | no write performed, no crash |
| 17 | `c2GJK` | `cache == NULL` | cache read and cache write-back both skipped |
| 18 | `c2GJK` | `cache != NULL` but `cache->count == 0` (`cache_was_good` false) | cache ignored on entry, simplex re-seeded from vertex 0; cache still written back on exit |
| 19 | `c2GJK` | cached simplex rejected: `min_metric < max_metric*2.0f && metric < -1.0e8f` (needs `metric < -1e8`, i.e. a huge negative determinant) | `cache_was_read` stays 0 → simplex re-seeded |
| 20 | `c2GJK` | cached simplex accepted (the common case, since `metric < -1e8f` is almost never true) | `cache_was_read = 1`, warm start from cached indices |
| 21 | `c2GJK` | `cache->iA[i]` / `cache->iB[i]` **within** the proxy's `count` range — C does no bounds check, so any index is accepted | warm start from those vertices; compared bit-for-bit across randomized caches. Indices ≥ `count` read an *uninitialised* `c2Proxy.verts[]` slot in the C (**UB**, the Rust zero-initialises), so they are documented and not compared. |
| 22 | `c2GJK` | `cache->count` negative | loop body never runs, `s.count` set negative → `c2GJKSimplexMetric` `default:`→0, `c2L`/`c2D`/`c2Witness` `default:` arms |
| 23 | `c2GJK` | `cache->count > 3` | writes past `c2Simplex`'s 4 `c2sv` slots / reads past `iA[3]` (**UB**, stack corruption). Documented, not exercised. |
| 24 | `c2GJK` | `cache->div == 0` on a warm start | `1.0f/0.0f = +inf` inside `c2Witness`/`c2L` → `inf`/`NaN` outputs; both libs must agree bitwise |
| 25 | `c2GJK` | GJK does not terminate early: loop guard `iter < 20` exhausted | **Verified unreachable** for this library: the largest proxy has 4 vertices, so the simplex converges or hits a duplicate support point almost immediately. A dedicated hunt over ~1.4M randomized configurations (all 9 type pairs × non-unit transforms × warm caches × 5 magnitude scales × both `use_radius`) observed a maximum of **3** iterations. `*iterations` is nevertheless compared bit-for-bit on every single `c2GJK` call in the suite. |
| 26 | `c2GJK` | `d1 > d0` (no progress) | `break` out of the loop |
| 27 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction degenerate, e.g. identical shapes) | `break` |
| 28 | `c2GJK` | duplicate support point (`iA==saveA[i] && iB==saveB[i]`) | `break` |
| 29 | `c2GJK` | `s.count == 3` after `c23` (origin enclosed → overlap) | `hit = 1`, `a = b`, returns `dist = 0.0f` |
| 30 | `c2GJK` | `use_radius != 0` and `!(dist > rA+rB && dist > FLT_EPSILON)` | `a = b = midpoint(a,b)`, returns `0.0f` |
| 31 | `c2GJK` | `use_radius != 0`, shrink performed, and `a.x==b.x && a.y==b.y` afterwards | `dist` forced to `0.0f` |
| 32 | `c2GJK` | `use_radius == 0` | raw core distance returned, radii ignored |
| 33 | `c2GJK` | `use_radius` a non-1 truthy int (e.g. `2`, `-1`, `INT_MIN`) | treated as true (`if (use_radius)`) |
| 34 | `c2Support` | `count <= 0` (0, negative) | `verts[0]` still read, loop skipped, returns `0` |
| 35 | `c2Support` | `d == (0,0)` — all dots equal, strict `>` never fires | returns `0` |
| 36 | `c2Support` | all dots are `NaN` — `dot > dmax` false for NaN | returns `0` |
| 37 | `c2Witness` | `s->count` not in {1,2,3} (0, negative, ≥4) — `default:` | `*a = *b = (0,0)` |
| 38 | `c2Witness` | `s->div == 0` | `den = +inf` → `inf`/`NaN` components, must match bitwise |
| 39 | `c2D` | `s->count == 3` or any other value — `case 3: default:` | returns `(0,0)` |
| 40 | `c2D` | `s->count == 2` and `c2Det2(ab, -a) == 0` (collinear with origin) — strict `> 0` fails | returns `c2CCW90(ab)` not `c2Skew(ab)` |
| 41 | `c2L` | `s->count` not in {1,2} — `default:` | returns `(0,0)` |
| 42 | `c2L` | `s->div == 0` with `count` 2 | `den = inf` → `inf`/`NaN`, must match bitwise |
| 43 | `c2GJKSimplexMetric` | `s->count` not 2 or 3 (`default:` falls into `case 1:`) | returns `0.0f` |
| 44 | `c2Div` | `b == 0` | `1.0f/0 = inf`; `inf * 0 = NaN`, `inf * x = ±inf` |
| 45 | `c2Norm` | `a == (0,0)` → `c2Len == 0` | `(NaN, NaN)` |
| 46 | `c2Norm` | `a` contains `inf` → `c2Len == inf` | `(NaN, NaN)` or `(0,±0)` per component; must match bitwise |
| 47 | `c2Len` | `c2Dot(a,a) < 0` impossible, but overflow to `+inf` for huge components | `sqrtf(inf) = inf` |
| 48 | `c2Len` | component `NaN` | `sqrtf(NaN) = NaN` |
| 49 | `c2CircletoCircle` | negative radii (`A.r + B.r < 0`) — `r2 = r2*r2` squares away the sign | behaves like the positive-sum radius |
| 50 | `c2CircletoAABB` | inverted AABB (`min > max`): `c2Clampv` = `max(lo, min(a, hi))` yields `lo` | still returns a well-defined 0/1 |
| 51 | `c2CircletoAABB` | `A.r < 0` — `r2 = A.r*A.r` positive | negative radius behaves like `|A.r|` |
| 52 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n == (0,0)`, `da == 0` so `da < 0` false, `db == 0` so `db < 0` false → distance to `B.b` | well-defined |
| 53 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n) == 0` | `da/0` → `NaN`/`inf` propagation, must match |
| 54 | `c2AABBtoAABB` | `NaN` coordinates — all four `<` comparisons false | returns `1` (reports overlap) |
| 55 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK` returns `NaN` — `if (NaN)` is true | returns `0` (no collision) |
| 56 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK` returns exactly `-0.0f` — `if (-0.0f)` is false, so the wrapper would report a collision | **Verified unreachable**: `dist` is either `c2Len(...)` (never negative zero), an explicit `0.0f`, or `dist - (rA+rB)` guarded by `dist > rA+rB` (strictly positive). A 180k-case scan over all type pairs with `use_radius=1` never produced `-0.0f`. |
| 57 | all `c2v` math | `±inf`, `NaN`, subnormal and `±0.0` inputs | IEEE-754 result, bit-for-bit identical (incl. `NaN` payload and sign of zero) |
| 58 | `c2Maxv` / `c2Minv` | `NaN` operand — ternary `a.x > b.x ? a.x : b.x` picks `b.x` for `NaN` | `NaN`-asymmetric; must not be replaced by `f32::max`/`min` |
| 59 | `c2Clampv` | `lo > hi` | `max(lo, min(a,hi))` == `lo` |
| 60 | `c2GJK` | `A`/`B` NULL with a valid type | C dereferences → segfault (UB); documented, not exercised |

## Status

Every row below is covered by a differential test in `tests/errors.rs` (with
help from `tests/configs.rs` for the rows that are also valid-path
configurations). All of them pass.

| rows | test |
|------|------|
| 1-4 | `rows1_4_c2Collided_bad_enums` |
| 6-8 | `rows6_8_omni_collide_bad_enums` |
| 9 | `row9_ptr_from_parts_bad_enum` (+ `configs.rs::row78_ptr_from_parts_valid`) |
| 10 | `row10_c2MakeProxy_bad_enum` |
| 11 | `row11_gjk_bad_enum_documented_ub` |
| 12-18 | `rows12_17_gjk_null_arguments` |
| 19-22, 24 | `rows19_22_24_gjk_crafted_caches` |
| 25-33 | `rows25_33_gjk_loop_exits_and_use_radius` |
| 34-36 | `rows34_36_c2Support_degenerate` |
| 37-43 | `rows37_43_simplex_out_of_range_counts` |
| 44-48 | `rows44_48_float_edge_cases` |
| 49-56 | `rows49_56_boolean_predicate_edges` |
| 57 | `rows44_48_float_edge_cases` + every `configs.rs` row (wild-float generator) |
| 58-59 | `rows58_59_minmax_clamp_nan_asymmetry` |

## Rows intentionally not executed

Rows 5, 23 and 60 describe genuine C undefined behaviour (NULL dereference,
out-of-bounds stack writes). Running them crashes or corrupts the test process
rather than producing a comparable result, so they are documented here and
skipped. Row 11 was attempted and confirmed to SIGSEGV inside the C library, so
it is handled the same way. Rows 9 and 21 are UB in the C but partially
reachable; the reachable part is compared and the equivalence argument for the
rest is recorded in the row.

## A note on NaN payloads

IEEE-754 leaves NaN payload propagation implementation-defined, and on x86-64 an
`addss`/`mulss` with two NaN operands returns the payload held in the
*destination* register. gcc `-O0` and LLVM pick different destination registers
for several expressions in this library, which surfaced as differing NaN
sign bits. `src/lib.rs` therefore performs every scalar float operation through
the `fp` module, which pins the destination operand with inline `asm!` to
exactly the instruction gcc emits (verified against `objdump -d` of the C
`.so`). All exported functions are also `#[inline(never)]` so that, as in the
`-O0` C build, each helper's arithmetic happens exactly once in one fixed form
rather than being re-scheduled per call site.
