# ERRORS.md — Phase A error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. First, the honest result of the
mechanical grep:

```sh
grep -nE 'return -1|return NULL|RETURN_ERROR|assert|errno|ERROR|E[A-Z]+_' c_src/src/lib.c
# -> no matches
```

**This library has no error codes, no error enum, no `assert`, and no
`return NULL`/`return -1` paths.** Every function returns unconditionally.
Its entire rejection surface is therefore made of:

* **null-pointer guards** (`if (!ax_ptr)`, `if (cache)`, `if (outA)`, ...) —
  the C substitutes a default or silently skips the write;
* **`switch` statements with a missing or catch-all `default`** — an
  out-of-range `C2_TYPE` / `count` is silently absorbed;
* **explicit range/degeneracy checks** (`<= 0`, `> 0`, `!!count`, `< FLT_EPSILON`,
  `iter < 20`, `d1 > d0`, `dup`) — the C *rejects* the iteration/branch and
  falls into a sentinel result;
* **sentinel returns** — `return 0`, `c2V(0, 0)`, `dist = 0` — which are this
  library's only way of reporting "degenerate / cannot compute";
* **division by a possibly-zero denominator** (`1.0f / s->div`,
  `c2Div(a, c2Len(a))`) — the C does not check, so `inf`/`NaN` *is* the
  documented-by-behaviour result and the Rust must produce the identical bits.

Constants that act as limits: `FLT_MAX` (`3.40282346...e+38F`, the `d0` seed),
`FLT_EPSILON` (`1.19209289550781250000000000000000000e-7F`), the literal
`-1.0e8f` in the cache test, the `20` iteration cap, `c2Proxy::verts[8]`,
`c2GJKCache::iA[3]`/`iB[3]`, and `c2Simplex`'s 4 `c2sv` slots.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `c2MakeProxy` | `type` is not 0/1/2 (`switch` has **no `default`**), e.g. `3`, `-1`, `INT_MAX`, `INT_MIN` | `*p` left **completely untouched** (caller's prior bytes preserved); no write at all | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count == 1` (explicit `case 1`) | returns `0.0f` | [x] |
| 3 | `c2GJKSimplexMetric` | `s->count` not in {1,2,3} (`default:`), e.g. `0`, `4`, `-1`, `INT_MAX` | returns `0.0f` | [x] |
| 4 | `c2D` | `s->count == 3` (explicit `case 3`) | returns `c2V(0,0)` | [x] |
| 5 | `c2D` | `s->count` not in {1,2,3} (`default:`), e.g. `0`, `4`, `-7` | returns `c2V(0,0)` | [x] |
| 6 | `c2L` | `s->count` not in {1,2} (`default:`), e.g. `0`, `3`, `4`, `-1` | returns `c2V(0,0)`; `den = 1/div` computed but unused | [x] |
| 7 | `c2L` | `s->div == 0` with `count == 2` | `den = +inf`; components become `±inf`/`NaN` — bit-exact match required | [x] |
| 8 | `c2Witness` | `s->count` not in {1,2,3} (`default:`), e.g. `0`, `4`, `-1` | `*a = *b = c2V(0,0)` | [x] |
| 9 | `c2Witness` | `s->div == 0` (`den = 1.0f/0.0f`) with `count` 2 or 3 | `inf`/`NaN` components written to `*a`,`*b` | [x] |
| 10 | `c2Witness` | `s->div == -0.0f` | `den = -inf`; sign-of-zero must propagate identically | [x] |
| 11 | `c2Support` | `count == 0` — loop `for(i=1;i<count;++i)` never runs but `verts[0]` is **still dereferenced** unconditionally | returns `0` (no rejection, no bounds check) | [x] |
| 12 | `c2Support` | `count < 0`, e.g. `-1`, `INT_MIN` | returns `0`, still reads `verts[0]` | [x] |
| 13 | `c2Support` | `d` is the zero vector, or all dots equal / `NaN` (`dot > dmax` is false for `NaN`) | returns `0` (first index wins ties and NaN) | [x] |
| 14 | `c2Div` | `b == 0.0f` | `c2Mulvs(a, +inf)` -> `±inf` or `NaN` (for `a == 0`) per component | [x] |
| 15 | `c2Div` | `b == -0.0f` | `1.0f/-0.0f = -inf`; signs must match bit-for-bit | [x] |
| 16 | `c2Norm` | `a` is the zero vector -> `c2Len(a) == 0` | `c2Div(a, 0)` -> `0 * inf` -> `NaN` in both components | [x] |
| 17 | `c2Norm` | `a` contains `NaN`/`inf` | `NaN` components (bit pattern must match) | [x] |
| 18 | `c2Len` | `a` contains `inf` -> `c2Dot` overflows to `+inf` | `sqrtf(+inf) = +inf` | [x] |
| 19 | `c2Len` | `a` components overflow the `f32` dot product | `+inf` (no check) | [x] |
| 20 | `c2GJK` | `typeA` out of the `C2_TYPE` enum range (C enums accept any `int`) | `c2MakeProxy` writes nothing -> `pA` is **uninitialised stack**; genuine C UB. Deterministic sub-case verified via row 1 instead | [x] (UB, see note) |
| 21 | `c2GJK` | `typeB` out of enum range | same as row 20 | [x] (UB, see note) |
| 22 | `c2MakeProxy` | enum value passed as a *negative* `int` and as `INT_MIN`/`INT_MAX` across FFI | no write (row 1); confirms the Rust `match` has no accidental extra arm | [x] |
| 23 | `c2GJK` | `ax_ptr == NULL` | `ax = c2xIdentity()` substituted | [x] |
| 24 | `c2GJK` | `bx_ptr == NULL` | `bx = c2xIdentity()` substituted | [x] |
| 25 | `c2GJK` | `outA == NULL` | result silently not written; return value still valid | [x] |
| 26 | `c2GJK` | `outB == NULL` | result silently not written | [x] |
| 27 | `c2GJK` | `iterations == NULL` | iteration count silently not written | [x] |
| 28 | `c2GJK` | `cache == NULL` | cache neither read nor written; fresh simplex seeded | [x] |
| 29 | `c2GJK` | `cache->count == 0` (`cache_was_good = !!cache->count` is false) | cache **not** read; fresh simplex; cache still written on exit | [x] |
| 30 | `c2GJK` | `cache->count != 0` and the validity test `!(min_metric < max_metric*2.0f && metric < -1.0e8f)` | `metric < -1.0e8f` is essentially never true, so `cache_was_read = 1` for **every** non-zero-count cache. The seeded simplex is trusted unconditionally. Replicate verbatim | [x] |
| 31 | `c2GJK` | `cache->count == 4` (past the 3-slot `iA`/`iB` arrays but still inside `c2Simplex`'s 4 slots) | reads `iA[3]`/`iB[3]` out of the declared array (adjacent struct bytes), seeds 4 verts, then every `switch` on `count` falls to `default` -> `c2L`=0, `c2D`=0, break, `c2Witness`=(0,0), `dist=0` | [x] |
| 32 | `c2GJK` | `cache->count < 0` | the `for (i = 0; i < cache->count; ++i)` loop body never executes; `s.count` set to the negative value; all `switch`es hit `default` -> `dist = 0`, `a = b = (0,0)` | [x] |
| 33 | `c2GJK` | `cache->div == 0` combined with a cache-seeded `count` of 2/3 | `1/0 = inf` propagates through `c2L`/`c2Witness` | [x] |
| 34 | `c2GJK` | `cache->metric` is `NaN` -> `min_metric`/`max_metric` comparisons all false | `cache_was_read = 1` (the `!( ... )` still holds) | [x] |
| 35 | `c2GJK` | `use_radius == 0` | radius shrink skipped entirely; raw simplex distance returned even when shapes overlap | [x] |
| 36 | `c2GJK` | `use_radius != 0` (incl. negative / `INT_MIN` — any non-zero) and `dist <= rA + rB` | collapse: `a = b = 0.5*(a+b)`, `dist = 0` | [x] |
| 37 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` | same collapse branch, `dist = 0` | [x] |
| 38 | `c2GJK` | `use_radius != 0`, shrink applied, and afterwards `a.x == b.x && a.y == b.y` | `dist` forced to `0` (but `a`,`b` keep the shrunk values, **not** the midpoint) | [x] |
| 39 | `c2GJK` | negative shape radius (`c2Capsule.r < 0`, `c2Circle.r < 0`) — never validated | `rA + rB` negative, `dist -= rA+rB` grows; points move the wrong way. Replicate | [x] |
| 40 | `c2GJK` | `s.count == 3` after `c23` -> `hit = 1` | loop breaks, `a = b`, `dist = 0` regardless of `use_radius` | [x] |
| 41 | `c2GJK` | `d1 > d0` (no progress / regression) | loop breaks early with the current simplex | [x] |
| 42 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction collapsed) | loop breaks | [x] |
| 43 | `c2GJK` | new support vertex duplicates a saved `(iA,iB)` pair -> `dup` | loop breaks **before** `++s.count`, so the appended vertex is discarded | [x] |
| 44 | `c2GJK` | `iter` reaches the hard cap `20` | `while (iter < 20)` exits; `*iterations == 20` | [x] |
| 45 | `c2GJK` | degenerate AABB (`min > max`, i.e. inverted/negative extent) — never validated | accepted; `c2BBVerts` produces a reversed winding, `area`/`c2Det2` sign flips | [x] |
| 46 | `c2GJK` | zero-extent AABB (`min == max`) / zero-length capsule (`a == b`) | duplicate support vertices -> `dup` break path | [x] |
| 47 | `c2GJK` | shape coordinates are `NaN` / `±inf` | every `<= 0` / `> 0` test is false for `NaN`, so `c22`/`c23` take their final `else` branch; `NaN` propagates to `dist` | [x] |
| 48 | `c2GJK` | `ax.r` / `bx.r` not a unit rotation (e.g. `c=0,s=0`, or huge values) — never validated | accepted; `c2Mulrv` scales/annihilates the vertices | [x] |
| 49 | `c22` | `a == b` -> `u == 0 && v == 0`; `v <= 0` wins | `count = 1`, `div = 1`, `a.u = 1` (first branch, not the `u <= 0` branch) | [x] |
| 50 | `c22` | `v <= 0` (origin beyond `a`) | collapse to vertex `a` | [x] |
| 51 | `c22` | `u <= 0` (origin beyond `b`) | `s->a = s->b`, collapse to vertex `b` | [x] |
| 52 | `c23` | `vAB <= 0 && uCA <= 0` | collapse to `a`, `count = 1` | [x] |
| 53 | `c23` | `uAB <= 0 && vBC <= 0` | `a = b`, `count = 1` | [x] |
| 54 | `c23` | `uBC <= 0 && vCA <= 0` | `a = c`, `count = 1` | [x] |
| 55 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0` | edge AB, `count = 2` | [x] |
| 56 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0` | `a = b; b = c`, edge BC, `count = 2` | [x] |
| 57 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0` | `b = a; a = c`, edge CA, `count = 2` | [x] |
| 58 | `c23` | degenerate triangle (`area == 0`, collinear/duplicate points) -> `uABC = vABC = wABC = 0` | falls through to the final `else`: `count = 3`, `div = 0` -> later `1/div = inf` | [x] |
| 59 | `c23` | any `NaN` coordinate -> all six `<=`/`>` tests false | final `else`, `count = 3`, `NaN` barycentrics | [x] |
| 60 | `c2Maxv` / `c2Minv` | one operand `NaN` (`a.x > b.x` false) | returns the **`b`** component (C ternary semantics) | [x] |
| 61 | `c2Clampv` | `lo > hi` (inverted range, never validated) | `c2Maxv(lo, c2Minv(a, hi))` -> returns `lo` | [x] |
| 62 | `c2Clampv` | `lo`/`hi` contain `NaN` | NaN-propagation per row 60 | [x] |
| 63 | `gjk` | `a == NULL` and/or `b == NULL` | forwarded as `outA`/`outB`; `c2GJK`'s null guards absorb them — **no crash** | [x] |
| 64 | `gjk` | `reverse` non-zero but not 1 (`2`, `-1`, `0x7f`, `0x80` truncated to `i8`) | `if (reverse)` is true for any non-zero `char` -> reversed argument order | [x] |
| 65 | `gjk` | `reverse` value whose low byte is zero (e.g. `256` truncated) | `char` truncation makes it `0` -> non-reversed order | [x] |
| 66 | `gjk` | `b5` (capsule radius) `== 0`, `< 0`, `NaN`, `inf` | no validation; propagates into row 36-39 logic | [x] |
| 67 | `c2Det2` / `c2Dot` | operands large enough to overflow `f32` | `±inf`, or `NaN` from `inf - inf` | [x] |
| 68 | `c2BBVerts` | inverted `bb` (`min > max`) | writes the 4 corners verbatim, reversed winding; no check | [x] |

### Note on rows 20/21 (out-of-range `C2_TYPE` reaching `c2GJK`)

`c2MakeProxy` has **no `default:` arm**, so for an out-of-range type it writes
nothing. Inside `c2GJK` the `c2Proxy pA; c2Proxy pB;` locals are *uninitialised
stack memory*, so the subsequent `pA.count` / `pA.verts[0]` reads are genuine
undefined behaviour in the C. That value is stack garbage that depends on the
caller's frame history and cannot be reproduced by any translation (the Rust
zero-initialises its proxies).

Those rows are therefore verified in the only way that is meaningful: the
*deterministic* half of the behaviour — "`c2MakeProxy` writes nothing for an
out-of-range type" — is asserted directly against a caller-owned, pre-poisoned
`c2Proxy` buffer (rows 1 and 22), for `3`, `4`, `-1`, `INT_MIN`, `INT_MAX` and
random out-of-range ints. `c2GJK` itself is only compared for the three valid
enum values. This is called out explicitly rather than silently skipped.

## Result

All 68 rows have a passing differential test in `tests/phase_c_errors.rs`
(33 tests), reinforced by `tests/nan_matrix.rs` (the exhaustive NaN/inf
cross-product) and `tests/fuzz_differential.rs`. Every assertion is bit-exact
(`f32::to_bits`), so `+0.0` vs `-0.0` and differing NaN payloads/sign bits are
failures, not passes. Coverage of the multi-way branches is *asserted*, not
assumed: `row30_row34_cache_validity_predicate`, `row49_row50_row51_c22_degenerate`,
`row52_to_row59_c23_all_branches_and_degenerate` and
`row40_row41_row42_row43_row44_loop_exits` recompute the C's own predicates and
fail if any branch was never taken.

### Divergence found and fixed

One real bug surfaced here — an entire *class* of bug, not a single case.

`MULSS`/`ADDSS`/`SUBSS`/`DIVSS` return their **destination** operand when both
operands are QNaNs. The C is compiled at `-O0` (no `CMAKE_BUILD_TYPE`), so gcc
emits one instruction per source operation with a fixed, per-expression choice of
destination register — and that choice is frequently the *second* source operand
(e.g. `c2Add` computes `b + a` at the instruction level; `c2Dot`'s second product
uses `b.y` as destination; `c2Witness`'s `den * u` uses `u`). LLVM at `-O2`
commutes `fmul`/`fadd`, folds `fneg` into `fsub`, and SLP-vectorises the two
lanes into `mulps`/`addps`. Every one of those rewrites is arithmetically
equivalent, and every one of them changes which NaN sign bit survives.

Since this library validates nothing — shape coordinates, radii and rotation
components may all be `NaN` — those are reachable inputs. Two source-level
attempts to block the folds failed (LLVM re-derives `fneg` from a sign-bit
`xor`, and `black_box` does not constrain operand order), so the fix pins the
instruction sequence: `src/lib.rs`'s `fp` module wraps `mulss`/`addss`/`subss`/
`divss`/`xorps` in inline asm with an explicit destination operand, and
`c2Mulvs`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Div`, `c2Mulrv`, `c2MulrvT`, `c2Add`,
`c22`, `c23`, `c2L`, `c2Witness` and `c2GJK` were transcribed to match gcc's
object code one instruction at a time. A portable `#[cfg]` fallback keeps the
crate building on non-x86-64 targets.

Affected functions before the fix: `c2MulrvT`, `c2Add` (and therefore `c2Mulxv`,
`c2Witness`, `c2L`, `c2GJK`), `c2Dot`, `c2Det2`, `c2Mulrv`, and both public
entry points `c2GJK`/`gjk`.

### Deliberately excluded (documented, not silently skipped)

Rows 20 and 21: an out-of-range `C2_TYPE` reaching `c2GJK` leaves its
`c2Proxy` locals uninitialised, so the C then reads stack garbage. That is
genuine C undefined behaviour whose value depends on the caller's frame history
and cannot be reproduced by any translation. The deterministic half of the
contract — `c2MakeProxy` performs no write at all for any out-of-range type — is
asserted exhaustively instead (rows 1 and 22: `3`, `4`, `5`, `7`, `99`, `-1`,
`-2`, `-100`, `INT_MIN`, `INT_MAX`, plus 3000 random out-of-range ints, against a
pre-poisoned caller-owned buffer).

A `cache->count` of 5 or more is also excluded: the C's seeding loop writes
`verts[4]`, which is past the end of `c2Simplex`'s four `c2sv` slots and
corrupts the caller's stack. Both implementations perform the identical
out-of-bounds write, so there is nothing to distinguish; `count == 4` (row 31) is
the largest value that stays inside the struct and *is* tested, including the
aliased `iA[3]`/`iB[3]` reads that land on `iB[0]` and `div`.
