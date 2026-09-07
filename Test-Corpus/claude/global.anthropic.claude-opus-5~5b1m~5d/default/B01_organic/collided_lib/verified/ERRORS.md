# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

Grep results that define the surface:

```
$ grep -n 'return\|assert\|NULL\|default' c_src/src/lib.c
  ... 82:            return 0;      <- collided: typeA==CIRCLE, typeB out of range
  ... 92:            return 0;      <- collided: typeA==AABB,   typeB out of range
  ... 96:    return 0;              <- collided: typeA out of range
```

There are **no** `assert`s, **no** `RETURN_ERROR`-style macros, **no** error
enums, **no** null-pointer checks, **no** range checks on any float, and **no**
min/max constants anywhere in the library. The *entire* rejection surface of the
library consists of the three `default: return 0;` arms of the switch nest in
`collided`, i.e. out-of-range `C2_TYPE` values. Every other function is total:
it accepts all 2^32 bit patterns per `float` argument and returns a value.

Two consequences that the tests must honour, both replicated in the Rust:

* An out-of-range tag is *silently* reported as "no collision" (`0`), which is
  indistinguishable from a valid non-colliding answer. The differential test
  therefore also pins the *pointers* — the C never dereferences them on the
  `default` path, so passing `NULL` there must still return `0` rather than
  crash, and the Rust must not dereference either.
* A valid tag that does *not* match the pointee's real type is **not** an error:
  the C blindly casts (`*(c2Circle *)A`). This is UB in C but perfectly
  well-defined at the ABI level (reinterpret the bytes), and the Rust mirrors it
  with `read_unaligned`. Covered as rows 8–9.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `collided` | `typeA` out of range (any value ∉ {0,1}: 2, 3, 7, 255, 0xFFFFFFFF, `INT_MIN` as u32, random) — `default` arm at lib.c:96 | returns `0`; `A`/`B` never dereferenced |
| 2 | `collided` | `typeA == C2_TYPE_CIRCLE (0)` **and** `typeB` out of range — inner `default` at lib.c:82 | returns `0`; `B` never dereferenced |
| 3 | `collided` | `typeA == C2_TYPE_AABB (1)` **and** `typeB` out of range — inner `default` at lib.c:92 | returns `0`; `B` never dereferenced |
| 4 | `collided` | **both** `typeA` and `typeB` out of range | returns `0` (outer `default` wins before `typeB` is examined) |
| 5 | `collided` | `A == NULL` with `typeA` out of range (pointer unused on the reject path) | returns `0`, no fault |
| 6 | `collided` | `B == NULL` with valid `typeA` but out-of-range `typeB` (pointer unused) | returns `0`, no fault |
| 7 | `collided` | both pointers `NULL` and both tags out of range | returns `0`, no fault |
| 8 | `collided` | tag/pointee **mismatch**: `typeA = C2_TYPE_AABB` but the buffer holds a 12-byte `c2Circle`; the extra 4 bytes are read as `max.y` | no error — reinterprets bytes; result must equal the Rust's bit-for-bit |
| 9 | `collided` | tag/pointee mismatch the other way: `typeA = C2_TYPE_CIRCLE` over a 16-byte `c2AABB` buffer (last 4 bytes ignored) | no error — reinterprets bytes; must match Rust |
| 10 | `collided` | **unaligned** pointers (`c2Circle`/`c2AABB` read from an odd byte offset) | no error — `-O0` gcc uses byte loads/`movss`; Rust uses `read_unaligned`; must match |
| 11 | `c2Dot` | `NaN` operand (quiet, e.g. `0x7FC00001`) in `a.x`, `a.y`, `b.x`, `b.y`, and all 15 non-empty subsets | no error — returns a NaN; the exact payload must match bit-for-bit |
| 12 | `c2Dot` | **signalling** `NaN` operand (`0x7F800001`) — SSE quiets it (`|= 0x400000`) but raises no trap | no error — returns quieted NaN, payload must match bit-for-bit |
| 13 | `c2Dot` | invalid-operation `0 * ±inf` (e.g. `a = (0, 0)`, `b = (inf, inf)`) | no error — default qNaN `0x7FC00000` |
| 14 | `c2Dot` | invalid-operation `(+inf) + (−inf)` (e.g. `a = (1, 1)`, `b = (inf, −inf)`) | no error — default qNaN `0x7FC00000` |
| 15 | `c2Dot` | overflow to `±inf` (e.g. `a = b = (3.4e38, 0)`) | no error — returns `+inf` |
| 16 | `c2Dot` | underflow / subnormal products (`1e-30 * 1e-30`) | no error — returns `+0.0` (gradual underflow) |
| 17 | `c2Maxv` / `c2Minv` | `NaN` in either component of either operand — the C ternary is **not** NaN-suppressing: `a>b?a:b` yields `b` whenever the comparison is unordered | no error — returns `b`'s component, never `a`'s; must match bit-for-bit |
| 18 | `c2Maxv` / `c2Minv` | signed-zero pair (`a = +0.0`, `b = -0.0` and vice versa) — `>`/`<` are false for equal values, so `b` is returned | no error — returns `b`'s component *including its sign bit* |
| 19 | `c2Clampv` | inverted interval `lo > hi` (the C performs no validation) | no error — `max(lo, min(a, hi))` collapses to `lo`; must match |
| 20 | `c2Clampv` | `lo`/`hi` containing `NaN` (no validation) | no error — the two non-suppressing ternaries compose; must match bit-for-bit |
| 21 | `c2CircletoCircle` | **negative** radius (`A.r = -1`) — no validation; `r2 = (A.r+B.r)²` squares away the sign | no error — must match |
| 22 | `c2CircletoCircle` | radii summing to `NaN` (`A.r = +inf`, `B.r = -inf`) ⇒ `r2 = NaN` ⇒ `d2 < NaN` is false | returns `0` |
| 23 | `c2CircletoCircle` | `NaN` centre coordinate ⇒ `d2 = NaN` ⇒ `NaN < r2` is false | returns `0` |
| 24 | `c2CircletoCircle` | `r == 0` for both circles (degenerate: `d2 < 0` is impossible) | returns `0` even for identical centres |
| 25 | `c2CircletoAABB` | inverted AABB (`min > max`) — no validation; `c2Clampv` returns `min` | no error — must match |
| 26 | `c2CircletoAABB` | `NaN` in `B.min` / `B.max` / `A.p` / `A.r` (all four independently) | no error — result driven by the non-suppressing clamp; must match |
| 27 | `c2CircletoAABB` | negative radius `A.r < 0` (`r2 = A.r²` is positive, so a negative radius *collides*) | no error — must match |
| 28 | `c2CircletoAABB` | `A.r = 0` (`d2 < 0` impossible) | returns `0` even for a centre inside the box |
| 29 | `c2AABBtoAABB` | inverted AABBs (`min > max` on either box) — no validation | no error — `!(d0\|d1\|d2\|d3)` must match |
| 30 | `c2AABBtoAABB` | `NaN` in any of the 8 coordinates — every `<` is false ⇒ all `dN = 0` ⇒ returns `1` | returns `1` (a NaN box "collides" with everything) |
| 31 | `c2AABBtoAABB` | exactly touching boxes (`A.max.x == B.min.x`) — `<` is strict | returns `1` (touching counts as collision) |
| 32 | `c2V` | any bit pattern incl. `NaN` payloads — pure field copy, must **not** quiet a signalling NaN | returns the bits unchanged |
| 33 | `c2Sub` | `inf − inf` on either component | no error — component becomes default qNaN `0x7FC00000` |
| 34 | `c2Sub` | `NaN` operand on either side of either component | no error — quieted NaN, payload must match bit-for-bit |
