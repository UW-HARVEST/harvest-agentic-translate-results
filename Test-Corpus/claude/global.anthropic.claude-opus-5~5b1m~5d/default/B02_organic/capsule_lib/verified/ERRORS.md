# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically. The greps used:

```sh
grep -nE 'assert|RETURN_ERROR|errno|NULL|return *-1|return *0;|goto'  c_src/src/lib.c
grep -nE 'if *\(!|is_null'                                            c_src/src/lib.c
grep -nE 'switch *\(|case |default:'                                  c_src/src/lib.c
grep -n  '/'                                                          c_src/src/lib.c   # divisions
grep -n  'sqrt'                                                       c_src/src/lib.c
```

**Result of the mechanical sweep:** this library contains

* **0** `assert`, `RETURN_ERROR`, `errno`, `goto`, `return -1`, `return NULL`;
* **0** explicit range / bounds / size checks;
* **2** null-pointer *checks* (`c2GJK`: `!ax_ptr`, `!bx_ptr`) plus **4** null-pointer
  *guards* on output params (`cache`, `outA`, `outB`, `iterations`);
* **8** `return 0;` statements, of which **7** are genuine *rejection* returns
  (`switch` `default:` arms + the "GJK distance was non-zero ⇒ not colliding"
  arms) and 1 is the `count<=1` metric result;
* **9** `switch` statements whose `default:`/fall-through arms define the
  behaviour for out-of-range enum/`count` values;
* **4** divisions and **1** `sqrtf` that are *unguarded* — degenerate input
  therefore has to produce the exact same `inf` / `NaN` / signed-zero bit
  pattern in Rust.

So the "error surface" is entirely made of **sentinel returns, `switch` default
arms, null-pointer branches and IEEE-754 degenerate results**. Every distinct
one is one row below.

Legend for "expected C result": `rc` = returned value; `s` = the `c2Simplex*`
argument; `p` = the `c2Proxy*` argument.

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|-------------------------------------------|-------------------|
| E1  | `c2Collided` | `typeA` not in `{0,1,2}` (e.g. `3`, `-1`, `0x7fffffff`, `INT_MIN`) — outer `switch` `default:` (L614) | `rc == 0`; neither `A` nor `B` dereferenced |
| E2  | `c2Collided` | `typeA == C2_TYPE_CIRCLE (0)` **and** `typeB` not in `{0,1,2}` — inner `default:` (L586) | `rc == 0` |
| E3  | `c2Collided` | `typeA == C2_TYPE_AABB (1)` **and** `typeB` not in `{0,1,2}` — inner `default:` (L598) | `rc == 0` |
| E4  | `c2Collided` | `typeA == C2_TYPE_CAPSULE (2)` **and** `typeB` not in `{0,1,2}` — inner `default:` (L610) | `rc == 0` |
| E5  | `c2MakeProxy` | `type` not in `{0,1,2}`; the `switch` at L114 has **no** `default:` label | *nothing at all is written to `*p`* — `p->radius`, `p->count`, `p->verts` keep their prior (caller-supplied) contents |
| E6  | `c2GJK` | `ax_ptr == NULL` (L368) | `ax = c2xIdentity()` = `{p={0,0}, r={c=1,s=0}}` is used instead of a fault |
| E7  | `c2GJK` | `bx_ptr == NULL` (L372) | `bx = c2xIdentity()` is used instead of a fault |
| E8  | `c2GJK` | `outA == NULL` (L510) | no store; `rc` still returned |
| E9  | `c2GJK` | `outB == NULL` (L512) | no store; `rc` still returned |
| E10 | `c2GJK` | `iterations == NULL` (L514) | no store; `rc` still returned |
| E11 | `c2GJK` | `cache == NULL` (L383 / L500) | cache block skipped entirely; `cache_was_read = 0` ⇒ simplex seeded from vertex 0 |
| E12 | `c2GJK` | `cache != NULL` **and** `cache->count == 0` ⇒ `cache_was_good == 0` (L384) | cache **not** read; simplex seeded from vertex 0. On return the cache is *overwritten* with `metric`/`count`/`iA`/`iB`/`div` |
| E13 | `c2GJK` | `cache != NULL`, `cache->count != 0`, and `metric >= -1.0e8f` (the normal case) ⇒ `!( … && metric < -1.0e8f)` is **true** (L405) | `cache_was_read = 1` — the *stale* cached simplex is accepted. (Quirk: the `metric < -1.0e8f` conjunct makes this branch practically unconditional.) |
| E14 | `c2GJK` | `cache->count != 0` and `metric < -1.0e8f` **and** `min_metric < max_metric*2` | `cache_was_read = 0` — cache rejected, simplex re-seeded |
| E15 | `c2GJK` | `cache->count < 0` (e.g. `-1`) | `cache_was_good = !!(-1) = 1`, but the `for (i=0;i<cache->count;…)` body never runs; `s.count = -1`, `s.div = cache->div`. `c2GJKSimplexMetric` hits `default:`→`case 1:`→`0`. Main loop `switch(-1)` → no arm; `s.count != 3`; `c2L` `default:`→`{0,0}`; `c2Witness` `default:`→`a=b={0,0}`; final cache write loop `i<-1` doesn't run. `rc == 0` (or the `use_radius` midpoint `0`) |
| E16 | `c2GJK` | `cache->count > 3` (e.g. `4`) | UB in C (writes past `c2sv d`). Excluded from differential testing — see note below |
| E17 | `c2GJK` | `cache->iA[i]` / `cache->iB[i]` in `[proxy.count, 8)` — e.g. `iA = 7` for a 1-vertex circle proxy (L387-390) | **no bounds check**. The read stays inside the `c2v verts[8]` array, but `c2Proxy pA;` (L376) is an *uninitialised* stack object and `c2MakeProxy` only writes `verts[0 .. count)`, so this reads **indeterminate** storage. UB — excluded, see note (measured: 5 of 7 indices return leftover stack garbage such as `0xf172b398` / `0x00005640`) |
| E18 | `c2GJK` | `iA`/`iB` ≥ 8 or < 0 | out-of-struct UB. Excluded — see note |
| E19 | `c2GJK` | `typeA` / `typeB` not in `{0,1,2}` ⇒ `c2MakeProxy` writes nothing ⇒ `c2Proxy pA;` (L376) is read **uninitialised** | UB in C (indeterminate stack). Excluded — see note |
| E20 | `c2GJK` | `use_radius != 0` **and** `dist <= rA+rB` (overlapping / touching) or `dist <= FLT_EPSILON` (L485) | `a = b = 0.5*(a+b)`, `rc == 0.0f` (`+0.0`) |
| E21 | `c2GJK` | `use_radius != 0`, `dist > rA+rB`, and after the radius shrink `a == b` exactly (L491) | `rc` forced to `0.0f` even though `dist-rA-rB > 0` was computed |
| E22 | `c2GJK` | simplex reaches `count == 3` ⇒ `hit = 1` (L441) | `a` overwritten with `b`; `rc == 0.0f`; the `use_radius` block is **skipped** |
| E23 | `c2GJK` | `rA`/`rB` negative (a capsule/circle with `r < 0`) and `use_radius != 0` | no validation — `dist -= rA+rB` *grows* `dist`; `rc` may exceed the true distance |
| E24 | `c2GJK` | `rA`/`rB` = `NaN`/`inf` and `use_radius != 0` | `dist > rA+rB` is `false` for `NaN` ⇒ midpoint branch ⇒ `rc == 0.0f`; for `+inf` also `false` ⇒ `rc == 0.0f` |
| E25 | `c2GJK` | any input coordinate is `NaN` | no validation; every comparison with `NaN` is `false`, so the `d1 > d0` / `dot > dmax` / `v <= 0` tests take their "else" edge. Must match bit-for-bit including the returned `NaN` payload |
| E26 | `c2GJK` | 20 iterations elapse without termination (`while (iter < 20)`, L425) | loop exits with `iter == 20`; `*iterations == 20`; witness computed from whatever simplex is current |
| E27 | `c2GJK` | duplicate support point found (`dup`, L471) | loop breaks **before** `++s.count`, so the freshly written `verts[s.count]` is left out of the simplex; `*iterations` < 20 |
| E28 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (L451) — search direction collapsed | loop breaks early |
| E29 | `c2GJK` | `d1 > d0` (L447) — distance stopped decreasing | loop breaks early |
| E30 | `c2Witness` | `s->div == 0.0f` (L312) ⇒ `den = 1.0f/0.0f = +inf` | no guard: `*a`/`*b` become `±inf`/`NaN` (`inf*0 = NaN`) |
| E31 | `c2Witness` | `s->div == -0.0f` | `den = -inf` |
| E32 | `c2Witness` | `s->count` not in `{1,2,3}` (0, 4, `-1`, `INT_MIN`) — `default:` (L332) | `*a = *b = {0,0}` |
| E33 | `c2L` | `s->div == 0.0f` (L347) | `den = +inf`, unguarded |
| E34 | `c2L` | `s->count` not in `{1,2}` (incl. `3`) — `default:` (L354) | returns `{0,0}` |
| E35 | `c2Div` | `b == 0.0f` (L339) | `1.0f/0 = +inf`; `a * inf` ⇒ `±inf` or `NaN` (for `a==0`) |
| E36 | `c2Div` | `b == -0.0f` | `1.0f/-0 = -inf` |
| E37 | `c2Norm` | `a == {0,0}` ⇒ `c2Len(a) == 0` (L343) | `c2Div(a, 0)` ⇒ `{0*inf, 0*inf}` = `{NaN, NaN}` |
| E38 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (huge coords) | `sqrtf(+inf) = +inf` |
| E39 | `c2Len` | any component `NaN` (L153) | `sqrtf(NaN)` = quieted `NaN` |
| E40 | `c2Support` | `count <= 0` (L298) | `verts[0]` is **still dereferenced** for `dmax`, loop body skipped, `rc == 0` |
| E41 | `c2Support` | `d == {0,0}` (all dots equal `0`, `dot > dmax` never true) | `rc == 0` (first vertex wins ties) |
| E42 | `c2Support` | `verts == NULL` | segfault in C (no guard). Excluded — see note |
| E43 | `c2GJKSimplexMetric` | `s->count` not in `{2,3}` — `default:` falls into `case 1:` (L162-164) | `rc == 0.0f` for `0`, `1`, `4`, `-1`, `INT_MIN` |
| E44 | `c2D` | `s->count == 3` or any other value — `case 3:`/`default:` (L292-294) | `rc == {0,0}` |
| E45 | `c2D` | `s->count == 2` and `c2Det2(ab, -a.p) == 0` or `NaN` (`> 0` false) (L288) | `c2CCW90(ab)` branch taken |
| E46 | `c22` | `v <= 0` (L191), incl. `v == -0.0` / `v == +0.0` | `s->count = 1`, `s->a.u = 1`, `s->div = 1` (vertex `a` kept) |
| E47 | `c22` | `v > 0 && u <= 0` (L195) | `s->a = s->b`, `count = 1` |
| E48 | `c22` | `u` or `v` is `NaN` | both `<= 0` tests false ⇒ **else** arm: `div = u+v` = `NaN`, `count = 2` |
| E49 | `c23` | `vAB<=0 && uCA<=0` (L222) | `count = 1`, keep `a` |
| E50 | `c23` | `uAB<=0 && vBC<=0` (L226) | `count = 1`, `a = b` |
| E51 | `c23` | `uBC<=0 && vCA<=0` (L231) | `count = 1`, `a = c` |
| E52 | `c23` | `uAB>0 && vAB>0 && wABC<=0` (L236) | `count = 2`, edge `AB` |
| E53 | `c23` | `uBC>0 && vBC>0 && uABC<=0` (L241) | `count = 2`, `a=b; b=c` |
| E54 | `c23` | `uCA>0 && vCA>0 && vABC<=0` (L248) | `count = 2`, `b=a; a=c` |
| E55 | `c23` | none of the above (incl. all-`NaN`) — `else` (L255) | `count = 3`, `div = uABC+vABC+wABC` |
| E56 | `c23` | degenerate triangle, `area == 0` ⇒ `uABC = vABC = wABC = ±0` | the three `<= 0` conjuncts are true ⇒ an earlier `count=2` arm is taken |
| E57 | `c2CircletoCapsule` | `B.a == B.b` (zero-length capsule) ⇒ `n = {0,0}`, `c2Dot(n,n) == 0` (L565) | `da == +0` so `da < 0` is false; `db == +0` so `db < 0` is false ⇒ the **`bp`** branch runs, *no* division happens ⇒ `rc` well-defined |
| E58 | `c2CircletoCapsule` | `B.a == B.b` but reached via `da >= 0 && db < 0` — impossible (both are `0`), so `0/0` is unreachable here | documents that the `da / c2Dot(n,n)` division is only reached with `c2Dot(n,n) > 0` |
| E59 | `c2CircletoCapsule` | `A.r + B.r < 0` (both radii negative) ⇒ `r*r > 0` | no validation: a negative total radius still yields a positive `r*r`, so the test can report a collision |
| E60 | `c2CircletoCapsule` | a `NaN` in `A.p`, `A.r` or `B.r` | `d2` or `r*r` becomes `NaN` ⇒ `d2 < r*r` is false ⇒ `rc == 0` |
| E60b | `c2CircletoCapsule` | a `NaN` in `B.a` / `B.b` only | **the `NaN` is discarded, NOT propagated.** `da` and `db` become `NaN`, so both `< 0` tests are false and the final `bp` arm runs — and `bp = A.p - B.b` / `d2 = dot(bp,bp)` involve neither `n` nor `ap`. So `rc` is a normal `0`/`1` computed from the finite data. *(Verified against the C; the naive "NaN always rejects" assumption is wrong here, so the test derives the expected value from a transliteration of the C body.)* |
| E61 | `c2CircletoCircle` | `A.r + B.r < 0` (L542-544) | `r2 = (A.r+B.r)^2 > 0`; still `rc == (d2 < r2)` — negative radii **not** rejected |
| E62 | `c2CircletoCircle` | `NaN` coordinate/radius | `d2 < r2` false ⇒ `rc == 0` |
| E63 | `c2CircletoAABB` | `B.min > B.max` (inverted AABB) (L548) | `c2Clampv` = `max(min, min(p, max))` ⇒ clamps to `B.min`; no validation, no error |
| E64 | `c2CircletoAABB` | `A.r < 0` | `r2 = A.r*A.r > 0` ⇒ can still report a collision |
| E65 | `c2CircletoAABB` | `NaN` in `A.p` or `A.r` | `d2` / `r2` become `NaN` ⇒ `d2 < r2` false ⇒ `rc == 0` |
| E65b | `c2CircletoAABB` | `NaN` in `B.max` | `c2Minv(A.p, B.max)` is `A.p.x < B.max.x ? A.p.x : B.max.x`; the comparison is false so `B.max.x` (`NaN`) is **selected** and propagates ⇒ `rc == 0` |
| E65c | `c2CircletoAABB` | `NaN` in `B.min` **only** | `c2Clampv` is `c2Maxv(lo=B.min, …)` and `c2Maxv` is `lo.x > other.x ? lo.x : other.x`; a `NaN` in the **first** operand loses the `>` and is **DISCARDED**. `rc` is a normal `0`/`1`. *(Verified against the C; the test derives the expected value from a transliteration of the C body rather than assuming propagation.)* |
| E66 | `c2AABBtoAABB` | inverted AABB (`min > max`) (L519-524) | pure comparison of the 4 half-plane tests; no validation |
| E67 | `c2AABBtoAABB` | **every** coordinate `NaN` | all four `<` are false ⇒ `d0|d1|d2|d3 == 0` ⇒ `rc == 1` (**reports a collision**) |
| E67b | `c2AABBtoAABB` | a **single** `NaN` coordinate | only the comparisons that coordinate participates in become false. A pair separated on *both* axes still has a true `<` from the other axis ⇒ `rc == 0`; poisoning the *only* true comparison (e.g. `A.max.x` or `B.min.x` for a pair separated on x alone) flips `rc` to `1`. *(Verified against the C; the test derives the expected value from a transliteration of the C body.)* |
| E68 | `c2AABBtoCapsule` | `c2GJK` returns non-zero (incl. `NaN`, which is "true" in C) (L528) | `rc == 0` |
| E69 | `c2AABBtoCapsule` | `c2GJK` returns exactly `0.0f` / `-0.0f` | `rc == 1` |
| E70 | `c2CapsuletoCapsule` | `c2GJK` returns non-zero (incl. `NaN`) (L534) | `rc == 0` |
| E71 | `c2CapsuletoCapsule` | `c2GJK` returns exactly `0.0f` | `rc == 1` |
| E72 | `c2BBVerts` | `bb->min > bb->max` | no validation; writes the 4 corners as-is |
| E73 | `capsule` | any of the 5 `float` args `NaN` / `±inf` / denormal (L619) | no validation; returns the 3-bit mask produced by the three `c2Collided` calls on those values |
| E74 | `capsule` | `r < 0` | no validation; the negative radius flows into `c2CircletoCapsule` / `c2GJK` |

## Rows deliberately excluded from differential testing

E16, E17, E18, E19 and E42 are **undefined behaviour in C** (out-of-bounds stack
writes / reads of indeterminate storage / a null dereference). The C
"expected result" is not a value the compiler is obliged to reproduce, so a
differential assertion against it is meaningless. They are recorded here for
completeness, and every *defined* neighbour of each is tested instead:

| excluded row | why it is UB | tested defined neighbour |
|--------------|--------------|--------------------------|
| E16 `cache->count > 3` | the `verts + i` writes run past `c2sv d` on the stack | E15 (`count < 0`), E12 (`count == 0`), and `count ∈ {1,2,3}` (Phase B row C63) |
| E17 `iA`/`iB` in `[proxy.count, 8)` | reads the indeterminate tail of the uninitialised `c2Proxy pA;` (L376) — `c2MakeProxy` only writes `verts[0 .. count)` | C63 drives `count ∈ {1,2,3}` with indices valid for the proxy |
| E18 `iA`/`iB` ≥ 8 or < 0 | reads/writes outside the `c2Proxy` struct entirely | as E17 |
| E19 invalid `typeA`/`typeB` into `c2GJK` | `c2MakeProxy` writes nothing, so the whole `c2Proxy` stays indeterminate | E1–E5 (invalid types through `c2Collided` / `c2MakeProxy`, both fully defined) |
| E42 `verts == NULL` in `c2Support` | null dereference for the initial `dmax` | E40 (`count <= 0`), E41 (`d == {0,0}`) |

Each exclusion was **verified empirically, not assumed**. A probe of E17 over
`iA ∈ 1..8` (circle-vs-circle, `cache->count = 1`) showed the C returning

```
iA=1: rc=0xffffff20 (NaN)  outA=(0xffffff20, 0xffffff20)
iA=2: rc=+inf              outA=(-1.2018e30, 3.094e-38)
iA=3: rc=8.246211          outA=(1, 2)            <- happens to alias verts[0]
iA=4: rc=+inf              outA=(-1.1964e30, 3.094e-38)
iA=5: rc=+inf              outA=(-1.2064e30, 3.094e-38)
iA=6: rc=8.246211          outA=(1, 2)
iA=7: rc=+inf              outA=(-1.2064e30, 3.094e-38)
```

— recognisable stack residue (leaked pointer fragments), differing between
indices that denote the same logically-absent vertex. Matching that is neither
possible nor meaningful, so E17/E18 are excluded on the same grounds as E16/E19.

## Coverage

Every non-excluded row (E1–E15, E20–E41, E43–E74, plus the E60b/E65b/E65c/E67b
refinements discovered while testing — **73 rows**) has a passing
differential test in `tests/phase_c_errors.rs`. See the checklist at the bottom
of that file's module docs, and `VERIFICATION.md` for the roll-up.
