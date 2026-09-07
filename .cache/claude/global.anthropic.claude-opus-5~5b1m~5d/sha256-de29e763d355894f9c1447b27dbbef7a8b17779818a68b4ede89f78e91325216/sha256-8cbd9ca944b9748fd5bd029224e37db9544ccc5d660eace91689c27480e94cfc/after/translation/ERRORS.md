# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return 0`,
`default:`, `assert`, `if (!...)`, null check, range check and min/max constant:

```sh
grep -n "assert\|RETURN_ERROR\|return -1\|return NULL\|return 0\|errno\|ERROR\|MAX\|MIN\|default:\|if (!" src/lib.c
```

The library has **no** error enum, **no** `assert`, **no** `errno` use and
**never** returns a negative/`NULL` sentinel. Its entire rejection surface is:

1. `default:`/unmatched-`switch` fall-through on an out-of-range `C2_TYPE`
   (C enums accept any `int`, so these are real reachable inputs),
2. `if (!ptr)` null-pointer substitution / null-guarded out-parameters,
3. IEEE-754 degenerate results (division by `div == 0`, `sqrtf` of a negative,
   NaN/Inf propagation) — the C "rejects" nothing here, it returns the NaN/Inf,
   and the Rust must return the identical bit pattern,
4. one hard-coded loop bound (`iter < 20`) and one fixed-capacity array
   (`c2Proxy.verts[8]`, `cache->iA[3]`).

| #  | function | trigger (the exact invalid input/condition) | expected C result | status |
|----|----------|---------------------------------------------|-------------------|--------|
| 1  | `c2MakeProxy` (l.114 `switch`, no `default:`) | `type` not in {0,1,2} — e.g. `3`, `-1`, `INT_MAX`, `INT_MIN` | **no store at all**; `*p` left completely untouched (caller's bytes survive) | [x] |
| 2  | `c2GJKSimplexMetric` (l.162 `default:` falls into `case 1:`) | `s->count` not in {2,3} — `0`, `1`, `4`, `-1`, `INT_MAX` | returns `0.0f` | [x] |
| 3  | `c2D` (l.293 `default:` shares `case 3:`) | `s->count` not in {1,2} — `0`, `3`, `4`, `-5` | returns `c2V(0,0)` | [x] |
| 4  | `c2Witness` (l.332 `default:`) | `s->count` not in {1,2,3} — `0`, `4`, `-1` | writes `*a = *b = c2V(0,0)`; **still evaluates `1.0f/s->div` first** (so `div==0` does not trap) | [x] |
| 5  | `c2L` (l.354 `default:`) | `s->count` not in {1,2} — `0`, `3`, `4`, `-1` | returns `c2V(0,0)`; `1.0f/s->div` still evaluated | [x] |
| 6  | `c2Collided` (l.586) | `typeA == C2_TYPE_CIRCLE` and `typeB` out of range | returns `0` | [x] |
| 7  | `c2Collided` (l.598) | `typeA == C2_TYPE_AABB` and `typeB` out of range | returns `0` | [x] |
| 8  | `c2Collided` (l.610) | `typeA == C2_TYPE_CAPSULE` and `typeB` out of range | returns `0` | [x] |
| 9  | `c2Collided` (l.614) | `typeA` out of range (any `typeB`, incl. also-invalid) | returns `0`, `B` never dereferenced | [x] |
| 10 | `c2GJK` (l.368 `if (!ax_ptr)`) | `ax_ptr == NULL` | substitutes `c2xIdentity()` — identical result to passing `&{{0,0},{1,0}}` | [x] |
| 11 | `c2GJK` (l.372 `if (!bx_ptr)`) | `bx_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| 12 | `c2GJK` (l.510 `if (outA)`) | `outA == NULL` | no store, no crash; return value still the distance | [x] |
| 13 | `c2GJK` (l.512 `if (outB)`) | `outB == NULL` | no store | [x] |
| 14 | `c2GJK` (l.514 `if (iterations)`) | `iterations == NULL` | no store | [x] |
| 15 | `c2GJK` (l.383 `if (cache)`) | `cache == NULL` | cache read **and** write-back both skipped; simplex initialised from vertex 0 | [x] |
| 16 | `c2GJK` (l.384 `cache_was_good = !!cache->count`) | non-NULL `cache` with `count == 0` | cache read skipped, but write-back at l.500 still happens (overwrites the caller's cache) | [x] |
| 17 | `c2GJK` (l.405, inverted validity test) | non-NULL cache with `count != 0`; because `metric < -1.0e8f` is essentially never true, `cache_was_read` is set to **1** for virtually every cache | the cached simplex is trusted and reused even when stale — must be reproduced, not "fixed" | [x] |
| 18 | `c2GJK` (l.386-397) | `cache->count > 3` (e.g. `4`) — loop `i < cache->count` walks past `cache->iA[3]` | reads adjacent bytes of the **caller-owned** cache (`iA[3]`≡`iB[0]`, `iB[3]`≡`div`) and writes `verts[3]` (`s.d`). Fully deterministic because the memory belongs to the caller; both impls agree bit-for-bit. (`saveA[3]` is written out of bounds too, but the loop always breaks before reading it, since `c2D`/`c2L` return `(0,0)` for `count==4`.) | [x] |
| 19 | `c2GJK` (l.389-390) | `cache->iA[i]`/`iB[i]` >= the proxy's `count` (e.g. index 3 on a CIRCLE proxy, `count==1`) | unchecked `pA.verts[iA]`. `c2Proxy pA;` is an **uninitialised local**, i.e. a fixed stack slot reused by every call from the same site, so this reads what an *earlier* call left there. Rust must NOT zero the proxy per call, must not bounds-check, and must not panic. Reproduced for the deterministic part; see **Scope** below | [x] |
| 20 | `c2GJK` (l.421-425 `while (iter < 20)`) | pathological shapes that never converge | loop hard-stops at `iter == 20`; `*iterations` is at most `20` | [x] |
| 21 | `c2GJK` (l.485) | `dist <= rA + rB` **or** `dist <= FLT_EPSILON`, with `use_radius != 0` | takes the else branch: `a = b = (a+b)*0.5f`, `dist = 0` | [x] |
| 22 | `c2GJK` (l.491) | after radius shrink, `a.x==b.x && a.y==b.y` | `dist` forced to `0` even though the subtraction produced non-zero | [x] |
| 23 | `c2GJK` (l.479 `if (hit)`) | simplex reached `count == 3` (shapes overlap) | `a = b`, `dist = 0`, `use_radius` branch skipped entirely | [x] |
| 24 | `c2GJK` (l.447 `if (d1 > d0)`) | non-decreasing distance (NaN inputs make this comparison false) | breaks out with the *previous* simplex; NaN input ⇒ never breaks here | [x] |
| 25 | `c2GJK` + NaN/Inf shape coords | any `c2v` field = NaN / +Inf / -Inf / -0.0 | no rejection: NaN propagates through `c2Len`→`sqrtf` and the returned `dist` is NaN with a specific bit pattern; `*iterations` and cache write-back must match | [x] |
| 26 | `c2Support` (l.298-308) | `count <= 0` (`0`, `-1`) | `verts[0]` is **still** dereferenced before the loop; loop body never runs; returns `0` | [x] |
| 27 | `c2Support` | `count > 8` (past `c2Proxy.verts`) | unchecked read of adjacent memory; returns an index ≥ 8. Must not panic | [x] |
| 28 | `c2Witness` / `c2L` (`1.0f / s->div`) | `s->div == 0.0f` (or `-0.0f`) | `den = +Inf` / `-Inf`; products become `NaN` (`0*Inf`) or `±Inf`; no trap, no error | [x] |
| 29 | `c2Div`, `c2Norm` | `b == 0.0f` / `c2Len(a) == 0` (zero vector) | `1.0f/0.0f = +Inf`; `c2Norm(c2V(0,0))` = `(NaN, NaN)` | [x] |
| 30 | `c2Len` | `c2Dot(a,a)` negative (only reachable via ±Inf ⇒ NaN, or NaN input) | `sqrtf` of NaN → quiet NaN (indefinite `0xFFC00000` for a non-NaN negative operand) | [x] |
| 31 | `c2CircletoCapsule` (l.565) | degenerate capsule `B.a == B.b` ⇒ `c2Dot(n,n) == 0` and `da == 0` (so `da < 0` false, `db < 0` false at `db==0`) | takes the `else` branch (`bp`), no division by zero; but `da>0, db<0` with `n==0` would divide by zero → `NaN` | [x] |
| 32 | `c2AABBtoAABB` / `c2CircletoAABB` | inverted AABB (`min > max`) | not validated: `c2Clampv` = `max(lo, min(a, hi))` silently yields `lo`; comparisons still evaluated | [x] |
| 33 | `c2AABBtoCapsule`, `c2CapsuletoCapsule` (l.528, l.534) | `c2GJK(...) != 0` (any non-zero distance, including **NaN**) | `if (nonzero) return 0;` — a NaN distance is truthy in C ⇒ returns `0`. Rust `!= 0.0` on NaN is also `true` ⇒ must match | [x] |
| 34 | negative radius | `c2Circle.r < 0`, `c2Capsule.r < 0`, `c2Proxy.radius < 0` | no validation; `r*r` makes it positive in `c2CircletoCircle`; in `c2GJK` `rA+rB` may be negative and *grow* the distance | [x] |
| 35 | `capsule` | any float args incl. NaN/Inf/±0/denormal | never fails; returns the 3-bit mask `0..7` | [x] |

## Scope: the one region that is not reproducible

Rows 18 and 19 both concern unvalidated `c2GJKCache` indices. They split into two
very different cases, and the distinction was established by measurement, not
assumption:

**(a) Deterministic and reproduced — index < the proxy `count` of an *earlier*
call.** `c2GJK`'s `c2Proxy pA; c2Proxy pB;` are uninitialised automatic
variables, so they live in a fixed pair of stack slots that every call from the
same call site reuses; `c2MakeProxy` writes only `verts[0 .. count)`. Reading
`verts[k]` with `k >= count` therefore returns the value an earlier call stored.
This is fully deterministic and the Rust reproduces it exactly: the proxies are
modelled as thread-local persistent scratch, not re-zeroed per call (see
`ProxyScratch` in `src/lib.rs`). Verified by hand-computation — priming with the
AABB `{(100,200),(300,400)}` and then reading indices 1/2/3 yields exactly
`(5.76923, 3.84615)`, `(4.2, 5.6)` and `(1.47059, 5.88235)`, i.e. corners 1/2/3
of the previous AABB, from BOTH libraries. Locked in by
`tests/phase_b_proxy_persistence.rs` (rows 69/70 of `CONFIGS.md`), which is
confirmed to fail if the per-call zeroing is reintroduced.

**(b) Not reproducible by construction — index into a slot that no preceding
call ever wrote.** Then the C reads stack residue left by *whatever ran last at
that depth*. In a differential harness that alternates `C.c2GJK` and
`R.c2GJK` from the same call site, the two frames overlap, so the C's own result
becomes a function of the *Rust* library's stack layout. Measured with
`probe2.c`:

| sequence | `iA[0]=1` | `iA[0]=2` | `iA[0]=3` |
|---|---|---|---|
| C alone | `(5.76923, 3.84615)` | `(4.2, 5.6)` | `(1.47059, 5.88235)` |
| Rust alone | `(5.76923, 3.84615)` | `(4.2, 5.6)` | `(1.47059, 5.88235)` |
| C, with a Rust call interleaved | `(0, 0)` | `(0, 0)` | `(0, 0)` |
| Rust, with a C call interleaved | `(5.76923, 3.84615)` | `(4.2, 5.6)` | `(1.47059, 5.88235)` |

The C is **not self-consistent** here: inserting an unrelated call changes its
answer. There is therefore no C behaviour to match, and no implementation can be
"correct". Consistent with this, the bundled `difftest/diff` harness ships
`NO_CACHE=1` and `CACHE_IDX0=1` knobs and comments the very line that generates
these indices with *"larger indices make the C read uninitialised stack"*. With
either knob set, `difftest/diff` reports **0 failures out of 21,300,002 checks**;
with neither, the only failures are in this region.

All other rows in this table are deterministic and are verified bit-for-bit.
