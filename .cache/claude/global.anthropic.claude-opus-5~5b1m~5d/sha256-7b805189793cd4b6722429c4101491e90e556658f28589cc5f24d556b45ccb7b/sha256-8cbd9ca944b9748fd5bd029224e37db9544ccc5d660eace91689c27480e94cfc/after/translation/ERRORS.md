# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Greps performed:

```
grep -n 'return -1\|return NULL\|return 0\|assert\|RETURN_ERROR\|errno' c_src/src/lib.c
grep -n 'if (!\|if (.*== 0\|<= 0\|== NULL\|default:' c_src/src/lib.c
grep -n 'switch' c_src/src/lib.c
```

**Finding:** the library has **no error codes, no `assert`, no `errno`, no
`NULL` returns and no negative sentinels.** Its entire rejection surface
consists of

1. **null-pointer guards** (`if (!ax_ptr)`, `if (outA)`, …) that substitute a
   default or silently skip an output, and
2. **`switch` statements whose `default:` / missing-`case` arms define the
   behaviour for out-of-range `int` values** (`C2_TYPE` is an unfixed enum and
   `c2Simplex::count` is a plain `int`, so both accept any `int` over FFI).

Every row below is one such distinct rejection/fallback branch. "expected C
result" is the *observable* result, which is what the differential test asserts.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `c2MakeProxy` | `type` matches no `case` in the `switch` (lib.c:109-129 has **no `default:`**). Tested: `3, 4, 7, -1, -999, INT_MIN, INT_MAX` | function is a no-op; `*p` left **completely unmodified** (caller-provided bytes preserved: `radius`, `count`, all 8 `verts`) |
| 2 | `c2MakeProxy` | `type == C2_TYPE_CIRCLE` (0) | writes `radius=c->r, count=1, verts[0]=c->p`; **`verts[1..8]` untouched** |
| 3 | `c2MakeProxy` | `type == C2_TYPE_AABB` (1) | writes `radius=0, count=4, verts[0..4]`; **`verts[4..8]` untouched** |
| 4 | `c2MakeProxy` | `type == C2_TYPE_CAPSULE` (2) | writes `radius=c->r, count=2, verts[0..2]`; **`verts[2..8]` untouched** |
| 5 | `c2GJKSimplexMetric` | `s->count` not in {2,3} — hits `default:` which falls into `case 1:` (lib.c:157-159). Tested: `0, 1, 4, 5, -1, INT_MIN, INT_MAX` | returns `0.0f` (positive zero, bit pattern `0x00000000`) |
| 6 | `c2D` | `s->count == 3` or any other value → `case 3: default:` (lib.c:287-289). Tested: `0, 3, 4, -1, INT_MIN, INT_MAX` | returns `c2V(0,0)` |
| 7 | `c2D` | `s->count == 1` with `a.p` = NaN / ±Inf / ±0 | returns `c2Neg(a.p)` = sign-flipped componentwise (`-0.0` for `+0.0`, NaN with flipped sign bit for NaN) |
| 8 | `c2D` | `s->count == 2` and `c2Det2(ab, -a.p)` is **NaN** ⇒ `> 0` is false | takes the `c2CCW90(ab)` branch, not `c2Skew(ab)` |
| 9 | `c2L` | `s->count` not in {1,2} → `default:` (lib.c:349-350). Tested: `0, 3, 4, -1, INT_MIN, INT_MAX` | returns `c2V(0,0)`, **regardless of `div`** (the `1.0f/div` is computed but unused) |
| 10 | `c2L` | `s->div == 0.0f`, `count == 2` | `den = 1.0f/0.0f = +Inf`; result propagates `±Inf`/`NaN` componentwise (no guard in C) |
| 11 | `c2L` | `s->div == -0.0f`, `count == 2` | `den = -Inf`; sign-propagating `±Inf`/`NaN` |
| 12 | `c2L` | `s->div == NaN`, `count == 2` | `den = NaN`; result NaN with C's NaN-propagation choice |
| 13 | `c2Witness` | `s->count` not in {1,2,3} → `default:` (lib.c:327-329). Tested: `0, 4, 5, -1, INT_MIN, INT_MAX` | `*a = *b = c2V(0,0)`; `den` computed from `div` but discarded |
| 14 | `c2Witness` | `s->div == 0` (any `count` in {1,2,3}) | `count==1` ⇒ unaffected (copies `sA`/`sB`); `count∈{2,3}` ⇒ `±Inf`/`NaN` outputs, no guard |
| 15 | `c2Support` | `count <= 0` (`0`, `-1`, `INT_MIN`) — C **unconditionally** reads `verts[0]` before the loop (lib.c:295) and the `for` never runs | returns `0`; requires `verts[0]` to be readable (test passes a valid 1-element buffer) |
| 16 | `c2Support` | all dots equal, or dots are NaN (`dot > dmax` always false) | returns `0` — the **first** index wins on ties/NaN, never a later one |
| 17 | `c2Support` | `d == c2V(0,0)` ⇒ every dot is `0.0` | returns `0` |
| 18 | `c2Norm` | `a == c2V(0,0)` ⇒ `c2Len == 0` ⇒ `c2Div(a, 0)` ⇒ `1.0f/0.0f = +Inf`, `0*Inf` | returns `c2V(NaN, NaN)` |
| 19 | `c2Norm` | `a` contains `Inf` ⇒ `len == Inf`, `1/Inf == 0` | `Inf*0 = NaN`, `finite*0 = ±0` — componentwise |
| 20 | `c2Norm` | `a` contains NaN | `sqrtf(NaN) = NaN`, `1/NaN = NaN` ⇒ `c2V(NaN, NaN)` |
| 21 | `c2Div` | `b == 0.0f` / `-0.0f` | `1.0f/b = ±Inf`, then `c2Mulvs` ⇒ `±Inf`/`NaN` componentwise |
| 22 | `c2Len` | `c2Dot(a,a) < 0` impossible for finite, but NaN input ⇒ `sqrtf(NaN)` | `NaN` (quiet, matching `sqrtf`) |
| 23 | `c2GJK` | `ax_ptr == NULL` (lib.c:363) | uses `c2xIdentity()` = `{p:(0,0), r:{c:1,s:0}}` instead of dereferencing |
| 24 | `c2GJK` | `bx_ptr == NULL` (lib.c:367) | uses `c2xIdentity()` |
| 25 | `c2GJK` | `outA == NULL` (lib.c:505) | witness point A silently discarded, no write, no crash |
| 26 | `c2GJK` | `outB == NULL` (lib.c:507) | witness point B silently discarded |
| 27 | `c2GJK` | `iterations == NULL` (lib.c:509) | iteration count silently discarded |
| 28 | `c2GJK` | `cache == NULL` (lib.c:378 and 495) | **both** the cache-read block and the cache-write block are skipped; a cold GJK is run |
| 29 | `c2GJK` | `cache != NULL` with `cache->count == 0` ⇒ `cache_was_good == 0` (lib.c:379) | cache read skipped, simplex seeded from vertex 0; cache **still written** on exit |
| 30 | `c2GJK` | `cache != NULL`, `cache->count != 0`, and the staleness test `!(min_metric < max_metric*2 && metric < -1.0e8f)` is **false** ⇒ `cache_was_read` stays 0 (lib.c:400). Note: `metric < -1.0e8f` is essentially never true, so this branch is *almost always* taken — a C quirk that must be replicated verbatim, **not** "fixed" | the just-built cached simplex is **discarded** and re-seeded from vertex 0 |
| 31 | `c2GJK` | `cache->count` negative (e.g. `-1`) ⇒ `!!count` is **1** (`cache_was_good`), the `for` loop body never runs, `s.count = -1`, `c2GJKSimplexMetric` hits `default:`→0 | reaches the main loop with `s.count == -1`: `save_count = -1`, the save loop is skipped, the `switch` matches no case, `c2L` `default:`⇒`(0,0)`, `d1 = 0`, `c2D` `default:`⇒`(0,0)`, `c2Dot(d,d)=0 < eps²` ⇒ **`break` on iteration 0**; then `c2Witness` `default:` ⇒ `a=b=(0,0)`, `dist=0`, `use_radius` ⇒ `dist(0) > rA+rB` false ⇒ midpoint branch ⇒ `dist=0`; cache written with `count=-1`, `iterations=0`. Returns `0.0f` |
| 32 | `c2GJK` | `use_radius == 0` | the whole radius-adjust block (lib.c:477-494) is skipped; raw `dist` returned, `a`/`b` are the raw witness points |
| 33 | `c2GJK` | `use_radius != 0` **and** `hit == 1` (simplex reached count 3) | `hit` branch wins: `a = b`, `dist = 0`; the radius block is `else if`, so **not** executed |
| 34 | `c2GJK` | `use_radius != 0`, `dist <= rA + rB` | midpoint branch: `a = b = (a+b)*0.5f`, `dist = 0` |
| 35 | `c2GJK` | `use_radius != 0`, `dist > rA+rB` **but** `dist <= FLT_EPSILON (1.1920929e-7)` | midpoint branch taken (the `&&` requires **both**), `dist = 0` |
| 36 | `c2GJK` | `use_radius != 0`, separated, and after shrinking `a.x==b.x && a.y==b.y` (lib.c:486) | `dist` forced to `0` even though the subtraction produced a nonzero value |
| 37 | `c2GJK` | `typeA` and/or `typeB` out of range ⇒ `c2MakeProxy` no-op ⇒ `pA`/`pB` hold whatever the **uninitialised stack** contained | genuinely indeterminate in C; **excluded from differential assertion** (documented UB, not a reproducible result). All *valid* enum values are covered in `CONFIGS.md`. |
| 38 | `c2GJK` | `A == NULL` / `B == NULL` with an **in-range** type ⇒ `c2MakeProxy` dereferences it | segfault in both C and Rust; **excluded from differential assertion** (documented UB) |
| 39 | `c2GJK` | `cache->count > 3` ⇒ writes `verts[i]` past `c2Simplex.d`, corrupting `div`/`count` and the caller's stack; and `cache->iA[i]` for `i>2` reads past `iA[3]` | UB / stack corruption; **excluded from differential assertion** |
| 40 | `c2GJK` | `cache->iA[i] >= pA.count` (but `< 8`) ⇒ reads `pA.verts[iA]`, which `c2MakeProxy` never initialised | reads uninitialised stack; **excluded from differential assertion** |
| 41 | `c22` | `v <= 0` where `v` is NaN ⇒ comparison false; then `u <= 0` also false | falls to the `else` (count 2) branch with NaN `u`/`v`/`div` |
| 42 | `c22` | both `u <= 0` and `v <= 0` (degenerate, `a == b == 0`) | the **first** branch (`v <= 0`) wins ⇒ `count = 1`, `a` kept (not `b`) |
| 43 | `c23` | any of the 7 branches with NaN barycentrics ⇒ all `<= 0` / `> 0` tests false | falls through to the final `else` ⇒ `count = 3` with NaN `u`s and `div` |
| 44 | `c23` | `area == 0` (collinear a,b,c) ⇒ `uABC = vABC = wABC = 0` | none of the `*ABC <= 0` guards can be reached before them, so branch order decides; if it reaches the final `else`, `div = 0` and `count = 3` |
| 45 | `gjk` | `reverse == 0` | calls `c2GJK(&bb, AABB, 0, &cap, CAPSULE, 0, a, b, 1, 0, 0)` — A is the box |
| 46 | `gjk` | `reverse != 0` (any nonzero `char`, incl. negative like `-128`, and `'\x01'`) | calls `c2GJK(&cap, CAPSULE, 0, &bb, AABB, 0, a, b, 1, 0, 0)` — A is the capsule (**operands swapped**, so `outA`/`outB` swap meaning) |
| 47 | `gjk` | `a == NULL` and/or `b == NULL` | forwarded as `outA`/`outB` to `c2GJK`, which null-checks them ⇒ silently no output, **no crash** |
| 48 | `gjk` | inverted AABB (`min > max`) | no validation anywhere in C; `c2BBVerts` builds a self-intersecting quad and GJK runs on it |
| 49 | `gjk` | negative capsule radius `b5 < 0` | no validation; flows into `pB.radius` and the `dist > rA+rB` test, which can then *grow* `dist` |
| 50 | `gjk` | non-finite floats (NaN / ±Inf) in any of the 9 float args | no validation; propagates through the whole solver including branch decisions |

## Rows excluded from assertion (documented C undefined behaviour)

Rows **37, 38, 39, 40** depend on uninitialised memory or out-of-bounds
access. Their C result is not a function of the inputs, so "C and Rust must
agree" is not a testable proposition for them. They are listed for
completeness and each has a `#[ignore]`d or commented test explaining why.
Every other row (1-36, 41-50) has an executing differential test.

## Status

All rows verified against both `.so`s by `tests/phase_c_errors.rs`:

* **rows 1-36, 41-50** — 36 executing differential tests, all passing.
* **rows 37-40** — `#[ignore]`d with an explanation; each depends on
  uninitialised memory or an out-of-bounds access, so the C result is not a
  function of the inputs and "C and Rust must agree" is not a testable
  proposition. The *well-defined* neighbours of each are asserted instead
  (e.g. the bad-enum no-op in row 1, and in-range cache indices in row 30).

Generic boundaries required by Phase C beyond the table, all covered:

| boundary | where |
|----------|-------|
| NULL pointers on every pointer parameter | rows 23-28 (`c2GJK`), row 47 (`gjk`), plus `c2MakeProxy(NULL, bad_enum, p)` in row 1 |
| zero lengths / counts | row 15 (`c2Support` with `count == 0`), row 29 (`cache->count == 0`) |
| oversized / negative lengths | row 15 (`count` down to `INT_MIN`), row 31 (`cache->count` negative), row 39 (`> 3`, documented UB) |
| one step past a valid range | rows 5, 6, 9, 13 (`count == 4` immediately past 3, and `count == 0` immediately below 1); row 35 (`dist` one ulp either side of `FLT_EPSILON`); row 30 (`metric` swept across the `-1.0e8f` threshold on both sides) |
| **out-of-range enum values across FFI** | row 1 — `C2_TYPE` given `3, 4, 5, 7, 100, -1, -2, -999, INT_MIN, INT_MAX, INT_MIN+1, INT_MAX-1`; also `use_radius` given `2, -1, 7, INT_MIN, INT_MAX` (row 32) and `gjk`'s `char reverse` given `2, 0x7f, -1, -128` (row 46) |

### Added rows: NaN-payload provenance

Fault injection revealed that "which of two NaNs survives" is a distinct,
observable rejection-adjacent behaviour that the original table did not pin.

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 51 | every two-operand leaf (`c2Add`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Mulvs`, `c2Div`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `c2Maxv`, `c2Minv`, `c2Clampv`) | **both** operands NaN, with distinct payloads/signs (incl. sNaN, which must be quieted by setting the mantissa MSB only) | the NaN of the operand that GCC placed in the `addss`/`subss`/`mulss` **destination** register, quieted; verified per site against the disassembly |
| 52 | `c2Witness`, `c2L` | `div` NaN (⇒ `den` NaN) **and** `u` NaN with a different payload | `u`'s NaN wins (`u` is the `mulss` destination), except at slot 0 where the enclosing `add_r` masks it |
| 53 | `c22`, `c23` | `u`/`v` (resp. `uABC`/`vABC`/`wABC`) simultaneously NaN with distinct payloads, reached via the NaN fall-through to the final `else` | the left addend wins (`add_l`) at all five `div` sum sites |
