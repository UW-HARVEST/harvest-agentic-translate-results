# ERRORS.md — Phase C error-surface table

## How this was derived

`c_src/src/lib.c` (530 lines) was grepped exhaustively for every rejection
mechanism:

```sh
grep -nE 'RETURN_ERROR|assert|return *-1|return *NULL|errno' c_src/src/lib.c   # -> 0 matches
grep -nE 'if *\(!?[a-z_]+\)'      c_src/src/lib.c   # null / truthiness guards
grep -nE 'switch|case |default:'  c_src/src/lib.c   # switch fallthrough surface
grep -nE '<=|>=|< *0|> *0|e\+38|e-7|1\.0e8|0\.5f|< 20|\[[0-9]\]' c_src/src/lib.c
```

**There is no error channel.** No `RETURN_ERROR` macro, no `assert`, no
`return -1`, no `return NULL`, no error enum, no `errno`. Every function returns
`void`, `float`, `int` (an index) or a struct by value, and *always* returns
something. So the rejection surface consists of exactly four kinds of row:

* **N** — explicit **n**ull-pointer guard the C performs (well defined).
* **S** — **s**witch with no matching `case` (out-of-range enum / count),
  reached through a `default:` or through the total absence of one.
* **G** — numeric **g**uard / boundary constant (well defined, incl. NaN-vs-`<=`).
* **U** — **u**nchecked dereference or unchecked array index: the C performs no
  check at all, so the input is *undefined behaviour*. Where the fault is
  deterministic (a null dereference) the row is still tested differentially by
  comparing the **termination signal of a subprocess**. Where the C's result
  depends on the *contents of uninitialised stack* it is marked
  `UB-indeterminate` and is deliberately NOT asserted — see "Untestable rows".

Legend for the last column: test function in `translation/tests/`.
`[x]` = row has a passing differential test.

## Table

| # | kind | function | trigger (exact invalid input / condition) | expected C result | [x] | test |
|---|------|----------|-------------------------------------------|-------------------|-----|------|
| 1 | N | `c2GJK` | `ax_ptr == NULL` (lib.c:363) | uses `c2xIdentity()`; identical result to passing `&{{0,0},{1,0}}` | [x] | `err_gjk_null_ax_is_identity` |
| 2 | N | `c2GJK` | `bx_ptr == NULL` (lib.c:367) | uses `c2xIdentity()` | [x] | `err_gjk_null_bx_is_identity` |
| 3 | N | `c2GJK` | `cache == NULL` (lib.c:378, 495) | cache-read block skipped (`cache_was_read = 0`) and no write-back; return value equals the `cache->count == 0` case | [x] | `err_gjk_null_cache` |
| 4 | G | `c2GJK` | `cache != NULL && cache->count == 0` (lib.c:379) | `cache_was_good = 0` -> warm start skipped, simplex re-seeded from vertex 0 | [x] | `err_gjk_cache_count_zero` |
| 5 | N | `c2GJK` | `outA == NULL` (lib.c:505) | `*outA` not written; caller's buffer byte-unchanged; return value unaffected | [x] | `err_gjk_null_outputs` |
| 6 | N | `c2GJK` | `outB == NULL` (lib.c:507) | `*outB` not written | [x] | `err_gjk_null_outputs` |
| 7 | N | `c2GJK` | `iterations == NULL` (lib.c:509) | `*iterations` not written | [x] | `err_gjk_null_outputs` |
| 8 | S | `c2MakeProxy` | `type == 3` (one past `C2_TYPE_CAPSULE`) | no `case` matches and there is **no `default:`** -> `*p` left **completely untouched** (all 72 bytes) | [x] | `err_makeproxy_out_of_range_enum` |
| 9 | S | `c2MakeProxy` | `type == -1` | `cmpl $2 / ja` is an **unsigned** test, so `-1` -> `0xffffffff > 2` -> untouched | [x] | `err_makeproxy_out_of_range_enum` |
| 10 | S | `c2MakeProxy` | `type == INT_MAX`, `INT_MIN`, `4`, `0x7fffffff`, `-2147483648` | untouched | [x] | `err_makeproxy_out_of_range_enum` |
| 11 | U/S | `c2MakeProxy` | `shape == NULL` **and** `type` out of range | `shape` is never dereferenced on the no-match path -> no fault, `*p` untouched | [x] | `err_makeproxy_null_shape_invalid_type` |
| 12 | S | `c2GJKSimplexMetric` | `s->count == 0` | `default:` falls through into `case 1:` -> returns `0.0f` | [x] | `err_simplexmetric_bad_count` |
| 13 | S | `c2GJKSimplexMetric` | `s->count == 1` | returns `0.0f` | [x] | `err_simplexmetric_bad_count` |
| 14 | S | `c2GJKSimplexMetric` | `s->count == 4` (one past the largest real count) | `default:` -> `0.0f` | [x] | `err_simplexmetric_bad_count` |
| 15 | S | `c2GJKSimplexMetric` | `s->count` = `-1`, `INT_MIN`, `INT_MAX` | `default:` -> `0.0f` | [x] | `err_simplexmetric_bad_count` |
| 16 | S | `c2D` | `s->count == 3` | `case 3:` shares the body of `default:` -> `c2V(0,0)` | [x] | `err_c2d_bad_count` |
| 17 | S | `c2D` | `s->count` = `0`, `4`, `-1`, `INT_MIN`, `INT_MAX` | `default:` -> `c2V(0,0)` | [x] | `err_c2d_bad_count` |
| 18 | S | `c2Witness` | `s->count == 4` (`cmp $3 / jg default`) | `*a = *b = c2V(0,0)`; note `den = 1/div` is still computed first | [x] | `err_witness_bad_count` |
| 19 | S | `c2Witness` | `s->count` = `0`, `-1`, `INT_MIN`, `INT_MAX` | `default:` -> `*a = *b = c2V(0,0)` | [x] | `err_witness_bad_count` |
| 20 | S | `c2L` | `s->count` = `0`, `3`, `4`, `-1`, `INT_MIN`, `INT_MAX` | `default:` -> `c2V(0,0)` (`den` still computed) | [x] | `err_c2l_bad_count` |
| 21 | S | `c2GJK` | simplex reaches the loop `switch` with `s.count ∉ {1,2,3}` (only reachable via a warm `cache->count == 4`) | no `default:` -> neither `c22` nor `c23` runs; loop then falls to `c2L`/`c2D`, both of which return `(0,0)` for that count, so `c2Dot(d,d) = 0 < FLT_EPSILON²` -> immediate `break` | [x] | `err_gjk_cache_count_four` |
| 22 | G | `c2Div` | `b == +0.0f` | `1.0f/0.0f = +inf`; `c2Mulvs` then yields `±inf` per component, or the x87/SSE default NaN `0xffc00000` for a `0 * inf` component | [x] | `err_div_by_zero` |
| 23 | G | `c2Div` | `b == -0.0f` | `1.0f/-0.0f = -inf` | [x] | `err_div_by_zero` |
| 24 | G | `c2Div` | `b == NaN` (quiet and signalling, both signs) | `divss` dst is the `1.0f` literal -> src NaN wins, quieted: `quiet(b)` | [x] | `err_div_by_zero` |
| 25 | G | `c2Norm` | `a == (0,0)` | `c2Len = 0` -> `1/0 = +inf` -> `(0*inf, 0*inf) = (NaN, NaN)` with the SSE default payload `0xffc00000` | [x] | `err_norm_degenerate` |
| 26 | G | `c2Norm` | `a` has an infinite component | `c2Len = +inf` -> `1/inf = 0` -> `(inf*0, …)` -> NaN / signed zero mix | [x] | `err_norm_degenerate` |
| 27 | G | `c2Witness` | `s->div == 0.0f` (`1.0f/div`) | `den = +inf`; each weight is scaled by `inf` | [x] | `err_witness_div_degenerate` |
| 28 | G | `c2Witness` | `s->div == NaN` | `den = quiet(div)`, propagated into every product | [x] | `err_witness_div_degenerate` |
| 29 | G | `c2L` | `s->div == 0.0f` / `NaN` / `±inf` | same `1.0f/div` degeneracy as row 27/28 | [x] | `err_c2l_div_degenerate` |
| 30 | G | `c2Support` | `count == 0` | loop never executes, **but `verts[0]` is still dereferenced** (lib.c:295) -> returns `0` | [x] | `err_support_nonpositive_count` |
| 31 | G | `c2Support` | `count < 0` (`-1`, `INT_MIN`) | `for (i = 1; i < count)` never runs -> returns `0`, after reading `verts[0]` | [x] | `err_support_nonpositive_count` |
| 32 | G | `c2Support` | every `c2Dot(verts[i], d)` is NaN (e.g. `d = (NaN, NaN)`) | `dot > dmax` is false for unordered -> returns `0` | [x] | `err_support_all_nan` |
| 33 | G | `c22` | `v` is NaN (so `v <= 0` is false) | falls through to the `u <= 0` test; with `u` NaN too, falls to the `else` arm -> `count = 2`, `div = add_l(u,v)` | [x] | `err_c22_nan_guards` |
| 34 | G | `c23` | any of `uAB,vAB,uBC,vBC,uCA,vCA,uABC,vABC,wABC` NaN | all six guarded arms are false for unordered compares -> final `else` -> `count = 3` | [x] | `err_c23_nan_guards` |
| 35 | G | `c2Maxv`/`c2Minv`/`c2Clampv` | either operand NaN | `comiss`+`jbe`, unordered takes the branch -> **always returns `b`** (`hi`/`lo` semantics invert) | [x] | `err_minmax_nan` |
| 36 | G | `c2GJK` | `cache->metric`/computed metric NaN, so `min < 2*max` is false | `!(false && …)` -> `cache_was_read = 1`, i.e. a NaN metric **accepts** the cache | [x] | `err_gjk_cache_nan_metric` |
| 37 | G | `c2GJK` | `metric >= -1.0e8f` (the `metric < -1.0e8f` half of lib.c:400 fails) | `cache_was_read = 1` — note this makes the guard accept virtually every cache | [x] | `err_gjk_cache_metric_threshold` |
| 38 | G | `c2GJK` | shapes so deeply overlapped that `s.count == 3` | `hit = 1` -> `a = b`, `dist = 0`, `use_radius` block skipped entirely | [x] | `cfg_gjk_overlap_hit` |
| 39 | G | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` | else-arm: `a = b = (a+b)*0.5f`, `dist = 0` | [x] | `cfg_gjk_radius_shrink` |
| 40 | G | `c2GJK` | `use_radius != 0` and `0 < dist <= FLT_EPSILON` | `dist > FLT_EPSILON` fails -> midpoint arm, `dist = 0` | [x] | `err_gjk_dist_below_epsilon` |
| 41 | G | `c2GJK` | `use_radius != 0`, `dist > rA+rB`, but `a == b` after the radius shift | `dist` forced back to `0` (lib.c:486) | [x] | `err_gjk_radius_collapse` |
| 42 | G | `c2GJK` | `use_radius == 0` | no radius adjustment at all; `dist` is the raw witness distance even for fat capsules | [x] | `cfg_gjk_use_radius_off` |
| 43 | G | `c2GJK` | iteration cap `while (iter < 20)` (lib.c:420) | `*iterations <= 20`, never more, even for adversarial inputs | [x] | `err_gjk_iteration_cap` |
| 44 | G | `c2GJK` | `d1 > d0` with `d1` NaN | `comiss`+`ja` is false when unordered -> **no** break; `d0` is then assigned NaN and every later compare also fails to break | [x] | `err_gjk_nan_shape_coords` |
| 45 | G | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (= `2^-46` exactly, `0x28800000`) | `break` out of the loop with the current simplex | [x] | `err_gjk_iteration_cap` |
| 46 | G | `c2GJK` | duplicate support point (`iA,iB` already in `saveA/saveB`) | `break` **after** the new vertex has been written to `verts[s.count]` but **without** incrementing `s.count` | [x] | `cfg_gjk_cache_roundtrip` |
| 47 | G | `gjk` | `reverse` low byte `== 0` (incl. `0x100`-style values truncated to `0`) | AABB is shape A, capsule is shape B | [x] | `cfg_gjk_wrapper_reverse` |
| 48 | G | `gjk` | `reverse` low byte `!= 0` (`1`, `-1`, `0x7f`, `0x80`) | capsule is shape A, AABB is shape B — outputs `a`/`b` swap meaning | [x] | `cfg_gjk_wrapper_reverse` |
| 49 | N | `gjk` | `a == NULL` and/or `b == NULL` | forwarded straight to `c2GJK`'s `outA`/`outB` guards -> **no fault** | [x] | `err_gjk_wrapper_null_outputs` |
| 50 | U | `c2BBVerts` | `out == NULL` | unchecked store -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 51 | U | `c2BBVerts` | `bb == NULL` | unchecked load -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 52 | U | `c2MakeProxy` | `shape == NULL`, `type` **in** range | unchecked load of `c->r` / `bb->min` -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 53 | U | `c2MakeProxy` | `p == NULL`, `type` in range | unchecked store -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 54 | U | `c2GJKSimplexMetric` | `s == NULL` | reads `s->count` -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 55 | U | `c22` | `s == NULL` | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 56 | U | `c23` | `s == NULL` | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 57 | U | `c2D` | `s == NULL` | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 58 | U | `c2L` | `s == NULL` | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 59 | U | `c2Witness` | `s == NULL` | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 60 | U | `c2Witness` | `s` valid with `count == 1`, `a == NULL` | `SIGSEGV` on `*a = …` | [x] | `err_null_deref_signals` |
| 61 | U | `c2Witness` | `s` valid with `count == 1`, `b == NULL` | `SIGSEGV` on `*b = …` | [x] | `err_null_deref_signals` |
| 62 | U | `c2Support` | `verts == NULL` (any `count`, incl. `0`) | `verts[0]` is read before the loop -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 63 | U | `c2GJK` | `A == NULL` with `typeA` in range | forwarded to `c2MakeProxy` -> `SIGSEGV` | [x] | `err_null_deref_signals` |
| 64 | U | `c2GJK` | `B == NULL` with `typeB` in range | `SIGSEGV` | [x] | `err_null_deref_signals` |
| 65 | S/G | `c2GJK` | `cache->count == 4` | `cache->iA[3]` aliases `cache->iB[0]`, `cache->iB[3]` aliases `cache->div`'s bits reinterpreted as `int`, and `verts + 3` is `s.d` — all still **inside** their structs, so this is fully defined and must match | [x] | `err_gjk_cache_count_four` |
| 37b | G | `c2GJK` | `min_metric == max_metric * 2.0f` **exactly**, with `metric < -1.0e8f` so the second conjunct does not mask the first (computed metric `-2.0e8f`, `cache->metric = -1.0e8f`) | `<` is false -> `cache_was_read = 1` (cache accepted); the distinction is observable in `*iterations` and the written-back cache | [x] | `err_gjk_cache_metric_double_boundary` |
| 66 | G | `c2GJKSimplexMetric` | `a.p` and `b.p` (and `c.p`) NaN with **distinct payloads** | pins the argument order of `c2Sub(b.p, a.p)` / `c2Det2`, which is otherwise unobservable because `|v| == |-v|` for every finite input | [x] | `err_simplexmetric_distinct_nan_payloads` |
| 67 | G | `c23` | `c2Det2(b,c)` and `area` both NaN with distinct payloads, reaching the final `else` arm (the only arm with no positivity guard on its operands) | pins the destination operand of `uABC = c2Det2(b,c) * area` and of `vABC`/`wABC` | [x] | `err_c22_c23_distinct_nan_payloads` |
| 68 | G | `c2Witness`, `c2L` | `s->div` NaN (so `den` is NaN) **and** a NaN vertex weight, distinct payloads | pins the destination operand of `den * u`. Only the LAST term of the `c2Add` chain is observable — see "Provably unobservable sites" below | [x] | `err_witness_c2l_distinct_nan_payloads` |

## Measured findings

Facts established by running the suite, not assumed:

* **`iter < 20` is unreachable.** `err_gjk_iteration_cap` reports a maximum of
  **4** iterations over ~6000 randomized shape/transform combinations. The three
  shape types have at most 4 proxy vertices, and the duplicate-support break
  (lib.c:459-467) fires long before the cap. The cap is therefore purely
  defensive: no input can distinguish `iter < 20` from `iter < 21`.
* **The `a == b` radius-collapse arm (row 41) IS reachable.** A directed search
  over circle/circle pairs found **123 genuine hits in 3038 candidates** that
  satisfy `dist > rA + rB` yet collapse to `a == b` after the shift, forcing
  `dist` back to `+0.0`. The search recomputes the C's own arithmetic through the
  exported leaf symbols, so a hit is a real hit and not an approximation.
* **`cache_was_read` is observable.** Warm-start and cold-start results differ in
  ~50% of randomized `count == 3` cache configurations (in `dist`, the witness
  points, `*iterations` and the written-back cache). GJK is self-correcting, so
  many configurations converge regardless — which is why rows 37/37b use a
  construction with all-integer coordinates: that makes the computed metric
  exactly `W*H` independent of shape position, decoupling "hit the constant
  exactly" from "make the outcome observable". 2048/3000 sampled positions are
  observable at the exact threshold.
* **`c2Support` never returns an out-of-range index**, asserted on every call in
  `cfg_support_counts`.
* **`c2GJK` never writes a cache `count` outside `1..=3`**, asserted on every
  generation in `cfg_gjk_cache_roundtrip`.

## Provably unobservable sites

These are places where two spellings of the C are indistinguishable *by
construction*, so no test can or should assert them. They are recorded so that a
future reader does not mistake the absence of a test for an absence of coverage.

| site | why unobservable |
|------|------------------|
| `c2Witness` / `c2L`: `den * u` for every term **except the last** of the `c2Add` chain | `c2Add` is `add_r` (destination = right operand), so when both operands are NaN the left term's payload is discarded. Both operands are NaN exactly when `den` is NaN, and that is exactly the only case in which `mul_l` and `mul_r` differ at all. Mutating the last term IS caught (`witness_v1/v2_den_mul_l`, `c2L_v1_den_mul_l`). |
| `c23`: `div = uAB + vAB`, `uBC + vBC`, `uCA + vCA` | those arms are guarded by `uXX > 0 && vXX > 0`, so neither operand can be NaN, and `add_l == add_r` for all non-NaN operands. The `uABC + vABC + wABC` sum in the final `else` has no such guard and IS caught. |
| `c2Support`: loop starting at `i = 1` vs `i = 0` | re-testing vertex 0 compares `dot > dmax` with `dot == dmax`, which is false; and false again when unordered. `imax` stays 0 either way. |
| `c2MulrvT(r, c2Neg(d))` vs `c2Neg(c2MulrvT(r, d))` in `c2GJK` | `c2MulrvT` is linear, so the two differ at most in a NaN payload/sign — and the value is consumed only by `c2Support`, which returns an *index*. Any NaN direction makes every `dot > dmax` false, so the index is 0 either way. |
| `iter < 20` vs any larger cap | unreachable, see "Measured findings". |
| `&&` operand order in `c23`'s guards | the operands are side-effect-free float comparisons. |
| `metric > metric_old` vs `>=` (and the `min` counterpart) | at equality both arms of the ternary return the same value. |

## Untestable rows (documented, deliberately not asserted)

These are inputs where the C's observable result is a function of
**uninitialised stack memory**. No implementation can be required to reproduce
them, so asserting equality would be asserting a coin flip. They are listed for
completeness, with the reason and the nearest *defined* row that IS tested.

| # | function | trigger | why not differentially testable | tested neighbour |
|---|----------|---------|---------------------------------|------------------|
| U1 | `c2GJK` | `typeA` or `typeB` out of `{0,1,2}` | `c2MakeProxy` has no `default:`, so `c2Proxy pA;` (lib.c:371, uninitialised) keeps stack garbage. `pA.count` and `pA.verts` are then read — indeterminate, and frequently a wild read. | row 8–11 test the same `c2MakeProxy` fallthrough directly, with a caller-owned zeroed `c2Proxy`, which *is* deterministic |
| U2 | `c2GJK` | `cache->iA[i] >= pA.count` (e.g. index 3 on a `C2_TYPE_CIRCLE`, whose `c2MakeProxy` writes only `verts[0]`) | reads `c2Proxy.verts[i]` slots that `c2MakeProxy` never wrote | `cfg_gjk_cache_warm_*` exercise every **in-range** index for each shape, incl. the max index 3 for `C2_TYPE_AABB` |
| U3 | `c2GJK` | `cache->iA[i] >= 8` or `< 0` | indexes outside `c2Proxy.verts[8]` altogether | as U2 |
| U4 | `c2GJK` | `cache->count >= 5` | `verts + 4` writes past `s.d`, i.e. past the end of `c2Simplex`, and `cache->iB[4]` reads past the end of the 36-byte `c2GJKCache` | row 65 tests `cache->count == 4`, the largest value that stays in bounds |

## Generic FFI-boundary boundaries (covered even though not in the table above)

* **Out-of-range enum values across FFI** — rows 8–11 (`c2MakeProxy`) and U1.
  A C enum is just `int`, so `3`, `-1`, `INT_MIN`, `INT_MAX` are all real
  inputs; the C's `cmpl $2 / ja` makes the test **unsigned**, which the Rust
  `match` on `c_int` with a `_ => {}` arm reproduces.
* **Null pointers** — rows 1–7, 11, 49 (checked by the C) and rows 50–64
  (unchecked; compared by subprocess exit signal).
* **Zero lengths / counts** — rows 4, 12, 30.
* **Oversized lengths / counts** — rows 14, 15, 17–20, 31, 65, U4.
* **One step past a valid range** — row 8 (`type = 3`), row 14
  (`count = 4` for the metric), row 18 (`count = 4` for `c2Witness`),
  row 65 (`cache->count = 4`).
* **Signalling vs quiet NaN, both signs, non-canonical payloads** — the
  randomised generator in `tests/common/mod.rs` emits `0x7f800001`,
  `0xffbfffff`, `0x7fc00000`, `0xffc00000` and random payloads, so rows 24,
  32–36 and 44 are each hit with many distinct payloads, and every Phase B row
  is additionally run with a NaN-heavy input distribution.
