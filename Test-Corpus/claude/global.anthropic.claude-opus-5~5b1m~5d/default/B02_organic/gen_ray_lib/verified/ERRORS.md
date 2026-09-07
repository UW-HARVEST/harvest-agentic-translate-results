# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Every `return 0`, every early-out
branch, every comparison that rejects an input, and every degenerate-arithmetic
sentinel (division by zero, `sqrtf` of a negative) is one row. The library has
no error enum, no `errno` use, no `assert`, and no `NULL` checks — the only
rejection channel is the `int` return value (`0` = miss / false) and the
degenerate float values written through `out`.

Grep receipts:

```sh
grep -n 'return 0'   c_src/src/lib.c   # 8 hits: L101 L109 L135 L146 L157 L195 L291 L302
grep -n 'return'     c_src/src/lib.c   # 33 hits
grep -n 'assert\|NULL\|errno\|-1' c_src/src/lib.c   # 0 hits (no assert/NULL check/errno)
```

Legend for "expected C result": `ret` is the `int` return value; `*out` says
whether the C wrote through the out pointer before returning.

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|-------------------------------------------|-------------------|
| 1  | `c2RaytoCircle` | L100 `disc < 0`, i.e. `c2Dot(m,m) - r*r > b*b` — ray line misses the circle entirely (also taken when `disc` is NaN? no: `NaN < 0` is false, see row 3) | `ret == 0`, `*out` untouched |
| 2  | `c2RaytoCircle` | L103 `t < 0` (`t = -b - sqrtf(disc)`) — circle is behind the ray origin | `ret == 0`, `*out` untouched |
| 3  | `c2RaytoCircle` | L103 `t > A.t` — hit exists but beyond the ray length | `ret == 0`, `*out` untouched |
| 4  | `c2RaytoCircle` | any input making `disc` NaN (e.g. `A.p`/`B.p`/`B.r`/`A.d` NaN or `inf-inf`): `disc < 0` false, so `sqrtf(NaN)`, then `t >= 0 && t <= A.t` both false | `ret == 0`, `*out` untouched |
| 5  | `c2RaytoCircle` | `A.t` = NaN with an otherwise valid hit: `t <= A.t` false | `ret == 0`, `*out` untouched |
| 6  | `c2AABBtoAABB` | L113 `B.max.x < A.min.x` (d0) — B entirely left of A | `ret == 0` |
| 7  | `c2AABBtoAABB` | L114 `A.max.x < B.min.x` (d1) — B entirely right of A | `ret == 0` |
| 8  | `c2AABBtoAABB` | L115 `B.max.y < A.min.y` (d2) — B entirely below A | `ret == 0` |
| 9  | `c2AABBtoAABB` | L116 `A.max.y < B.min.y` (d3) — B entirely above A | `ret == 0` |
| 10 | `c2AABBtoAABB` | **all** compared coordinates NaN: all four `<` are false, so the C **accepts**. Note (verified by test): a *single* NaN coordinate only neutralises the two comparisons that read it — e.g. `B = {min:(NaN,0), max:(-20,10)}` against `A = {(0,0),(10,10)}` is still rejected by d0 (`-20 < 0`). | all-NaN: `ret == 1`; single NaN: per the remaining comparisons |
| 11 | `c2RaytoAABB` | L145 `!c2AABBtoAABB(a_box, B)` — ray's own bounding box misses B | `ret == 0`, `*out` untouched |
| 12 | `c2RaytoAABB` | L156 `d > 0` — separating-axis reject on the ray's normal axis | `ret == 0`, `*out` untouched |
| 13 | `c2RaytoAABB` | L194 `hit == 0`, i.e. all four `t0..t3 > 1.0f`. Verified unreachable with finite inputs (see the note below); it needs all four `da` to be NaN, which needs a NaN in **both** lanes of `A.p` (`da0`/`da1` read `p0.x`, `da2`/`da3` read `p0.y`). | `ret == 0`, `*out` untouched |
| 14 | `c2RaytoAABB` | inverted box `B.min > B.max` (invalid AABB, no validation in C) | whatever the arithmetic yields; must match bit-for-bit |
| 15 | `c2RaytoAABB` | `A.t` NaN / `A.d` NaN so `p1` is NaN (but `A.p` stays finite): `c2AABBtoAABB` accepts (row 10) and `d` is NaN so `d > 0` is false — but `da0..da3` are computed from the **finite `p0`**, so they are still `< 0`, every `t` is `0` and `hit` is `1`. Corrected from the initial guess of `ret == 0`. | `ret == 1`, `out->t = 0 * A.t` (NaN when `A.t` is NaN), `out->n` = a face normal |
| 16 | `c2RayToPlane_OneDimensional` (static) | L126 `da < 0` — point already behind the plane | returns `0.0f` |
| 17 | `c2RayToPlane_OneDimensional` (static) | L133 `d == 0` (`da == db`, ray parallel to the plane) | returns `0.0f` |
| 18 | `c2RayToPlane_OneDimensional` (static) | `da`/`db` NaN: `da<0` false, `da*db>0` false, `d = NaN != 0` true | returns `NaN` (`da/d`) |
| 19 | `c2AABBtoPoint` | L218 `B.x < A.min.x` (d0) | `ret == 0` |
| 20 | `c2AABBtoPoint` | L219 `B.y < A.min.y` (d1) | `ret == 0` |
| 21 | `c2AABBtoPoint` | L220 `B.x > A.max.x` (d2) | `ret == 0` |
| 22 | `c2AABBtoPoint` | L221 `B.y > A.max.y` (d3) | `ret == 0` |
| 23 | `c2AABBtoPoint` | `B` NaN: all four comparisons false | `ret == 1` (accepts!) |
| 24 | `c2CircleToPoint` | L228 `d2 >= A.r * A.r` — point on or outside the circle (note: **strict** `<`, so a point exactly on the rim is rejected) | `ret == 0` |
| 25 | `c2CircleToPoint` | negative radius `A.r < 0` (invalid): `r*r` is positive, so it behaves like `|r|` | `ret` per `d2 < r*r` |
| 26 | `c2CircleToPoint` | `A.r` = NaN or point NaN: `d2 < r*r` false. Also `A.r = inf` with a far-away point: `d2` overflows to `+inf` too and `inf < inf` is false (strict `<`), so it still **rejects**; `A.r = inf` only accepts when `d2` stays finite. | `ret == 0` |
| 27 | `c2RaytoCapsule` | L291 fall-through: `yAe.x*yAp.x >= 0` **and** `min(|yAe.x|,|yAp.x|) >= B.r` — ray stays on one side, never within the slab | `ret == 0`, **`*out` HAS been written** (L243-244 set `out->n = c2Norm(cap_n)`, `out->t = 0`) |
| 28 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b`: `c2Norm` of the zero vector -> `c2Len` = 0 -> `c2Div` by 0 -> `1/0 = +inf`, `0*inf = NaN`. `M.y`/`out->n` become NaN | `ret` and `*out` per the NaN arithmetic; must match bit-for-bit |
| 29 | `c2RaytoCapsule` | `B.r < 0` (invalid radius): `capsule_bb.min.x = -B.r > 0 = capsule_bb.max.x`, an inverted box, so `c2AABBtoPoint` rejects | must match bit-for-bit |
| 30 | `c2RaytoCapsule` | L278 `d = yAe.x - yAp.x == 0` (no division guard here, unlike L133): `t = (c - yAp.x)/0` -> `±inf` or NaN | must match bit-for-bit |
| 31 | `c2RaytoCapsule` | delegates to `c2RaytoCircle`, which may reject (rows 1-5) | `ret == 0` from the delegate, `*out` still holds the L243-244 values |
| 32 | `c2CastRay` | L302 unreachable `return 0` after `case C2_TYPE_CAPSULE` (dead code — the preceding `return` always fires) | never observable |
| 33 | `c2CastRay` | **`typeB` not in {0,1,2}** (e.g. `3`, `-1`, `INT_MAX`, `INT_MIN`): the `switch` has no `default:` and the function has no trailing `return`, so control **falls off the end** — UB. The compiled C jumps straight to `leave; ret`, returning whatever the caller happened to leave in `%eax`. | **UB / unspecified `int`**; `*out` untouched; must not crash. See note below. |
| 34 | `c2Div` / `c2Norm` | `b == 0.0f` / zero-length vector: `1.0f/0.0f = +inf`, then `0*inf = NaN`, `x*inf = ±inf` | `{NaN or ±inf, ...}`, no trap |
| 35 | `c2Div` / `c2Norm` | `b == -0.0f`: `1.0f/-0.0f = -inf` | `-inf`-scaled result |
| 36 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `a.x = 1e38`) | `sqrtf(+inf) = +inf` |
| 37 | `c2Len` | `c2Dot(a,a)` is NaN (e.g. `a.x = inf, a.y = 0` gives `inf`; `a.x = inf, a.y = NaN` gives NaN) | `sqrtf(NaN) = NaN` |
| 38 | `c2Len` | `c2Dot(a,a) < 0` is impossible for real values, but reachable with NaN inputs; `sqrtf` of a negative would give the `-NaN` indefinite | matches `sqrtss` |
| 39 | `gen_ray` | `mp == ray.p` (zero ray direction): `c2Norm` of the zero vector -> `ray.d` NaN and `ray.t` NaN. The circle cast rejects (row 4) and the capsule cast falls through (row 27), but the **AABB cast still HITS** for exactly the reason in row 15. Corrected from the initial guess of `ret == 0`. | `ret == 4` (bit 2 only); `cast3->t` = `0 * NaN` = NaN |
| 40 | `gen_ray` | all misses | `ret == 0` |
| 41 | `gen_ray` | out pointers: the C **always** dereferences `cast2` (via `c2RaytoCapsule` L243-244) but only conditionally `cast1`/`cast3` | passing NULL for `cast2` always faults; NULL `cast1`/`cast3` faults only on hit |
| 42 | all `*out` functions | `out == NULL` on a path that writes: C segfaults (no check) | SIGSEGV — not asserted as a value; tested only for the paths where C provably does **not** write |

## Note on row 13 (unreachability with finite inputs)

`c2RayToPlane_OneDimensional` returns `0.0f`, `1.0f`, or `da / (da - db)` in the
branch where `da >= 0` and `da * db <= 0`; there `d = da - db >= da >= 0`, so the
quotient is always `<= 1`. Hence `tN <= 1.0f` — and `hitN == 1` — for every
finite input, and `hit == 0` is reachable **only** when some `tN` is NaN
(`NaN <= 1.0f` is false). The differential test for this row therefore lives in
the random-bit-pattern fuzz row (`row44_aabb_random_bits`, which hits it 570
times out of 20000) as well as in the explicit NaN case in
`errors_rows11_15_aabb_rejections`.

## Note on row 33 (out-of-range `C2_TYPE`)

This is the one row where a bit-exact return value cannot be asserted: the C
falls off the end of a non-`void` function, which is undefined behaviour. The
generated code (`ja .Lret` straight to `leave; ret`) returns the caller's
leftover `%eax`, which is not a property of the library at all — it depends on
the calling code. The differential test therefore asserts the parts that *are*
observable and well-defined:

* neither library crashes,
* neither library writes through `out` (compared bit-for-bit against the
  pre-call contents),

and records the two return values without requiring equality. Making the Rust
return a fabricated constant here would not increase fidelity, since no
constant can match "whatever was in `%eax`".

## Row status

All rows are exercised by `translation/tests/errors.rs`.

- [x] 1  [x] 2  [x] 3  [x] 4  [x] 5  [x] 6  [x] 7  [x] 8  [x] 9  [x] 10
- [x] 11 [x] 12 [x] 13 [x] 14 [x] 15 [x] 16 [x] 17 [x] 18 [x] 19 [x] 20
- [x] 21 [x] 22 [x] 23 [x] 24 [x] 25 [x] 26 [x] 27 [x] 28 [x] 29 [x] 30
- [x] 31 [x] 32 (dead code, documented) [x] 33 (as scoped above) [x] 34 [x] 35
- [x] 36 [x] 37 [x] 38 [x] 39 [x] 40 [x] 41 [x] 42
