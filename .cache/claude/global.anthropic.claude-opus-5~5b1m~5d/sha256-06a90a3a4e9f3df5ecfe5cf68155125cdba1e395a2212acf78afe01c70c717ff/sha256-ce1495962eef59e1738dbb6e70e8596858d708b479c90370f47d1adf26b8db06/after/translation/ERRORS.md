# ERRORS.md — error / rejection surface (Phase A)

The library has **no error codes, no `errno`, no asserts, no allocation and no
`RETURN_ERROR`-style macro**. Its entire rejection surface consists of

* the `return 0` (= "no hit" / "no overlap") statements of the predicate and
  raycast functions,
* the degenerate arithmetic paths (`/ 0`, `1.0f / 0`, `sqrtf` of NaN, NaN
  comparisons that make every `<`/`>`/`>=` false),
* one **switch with no `default` label** (`c2CastRay`), which for an
  out-of-range `C2_TYPE` falls off the end of a non-`void` function.

Every row below was derived by grepping `c_src/src/lib.c` for `return 0`,
`return`, `if (`, `? :` and `switch` (see the grep in the session log), one row
per distinct rejecting condition.

Line numbers refer to `c_src/src/lib.c`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
|  1 | `c2RaytoCircle` | L100 `disc < 0` — ray line misses the circle (`b*b - c < 0`) | returns `0`, **`*out` untouched** | `err_01_raytocircle_disc_negative` | [x] |
|  2 | `c2RaytoCircle` | L100 `disc` is NaN (`B.r` or ray NaN) → `disc < 0` false, then `t` NaN → `t >= 0` false | returns `0`, `*out` untouched | `err_02_raytocircle_nan_disc` | [x] |
|  3 | `c2RaytoCircle` | L103 `t < 0` — impact behind the ray origin (ray starts past the circle) | returns `0`, `*out` untouched | `err_03_raytocircle_t_negative` | [x] |
|  4 | `c2RaytoCircle` | L103 `t > A.t` — hit farther away than the ray length (incl. `A.t = 0`, `A.t < 0`, `A.t = NaN`) | returns `0`, `*out` untouched | `err_04_raytocircle_t_beyond_len` | [x] |
|  5 | `c2RaytoCircle` | `out == NULL` **and** the call rejects (rows 1–4) → the C never dereferences `out` | returns `0`, no crash | `err_05_raytocircle_null_out_on_miss` | [x] |
|  6 | `c2AABBtoAABB` | L113 `d0`: `B.max.x < A.min.x` | returns `0` | `err_06_aabbtoaabb_d0` | [x] |
|  7 | `c2AABBtoAABB` | L114 `d1`: `A.max.x < B.min.x` | returns `0` | `err_07_aabbtoaabb_d1` | [x] |
|  8 | `c2AABBtoAABB` | L115 `d2`: `B.max.y < A.min.y` | returns `0` | `err_08_aabbtoaabb_d2` | [x] |
|  9 | `c2AABBtoAABB` | L116 `d3`: `A.max.y < B.min.y` | returns `0` | `err_09_aabbtoaabb_d3` | [x] |
| 10 | `c2AABBtoAABB` | any coordinate NaN → all four `<` false → `!(0)` | returns `1` (**accepts**, a NaN box "overlaps") | `err_10_aabbtoaabb_nan_accepts` | [x] |
| 11 | `c2RaytoAABB` | L145 swept-AABB vs box rejected by `c2AABBtoAABB` | returns `0`, `*out` untouched | `err_11_raytoaabb_broadphase_reject` | [x] |
| 12 | `c2RaytoAABB` | L156 `d > 0` — separating-axis reject on the ray's normal | returns `0`, `*out` untouched | `err_12_raytoaabb_sat_reject` | [x] |
| 13 | `c2RaytoAABB` | L174/194 `hit == 0` (all four `t_i > 1.0f`, e.g. every `da*db > 0` and `da/(da-db) > 1`) | returns `0`, `*out` untouched | `err_13_raytoaabb_no_plane_hit` | [x] |
| 14 | `c2RaytoAABB` | `out == NULL` and the call rejects (rows 11–13) | returns `0`, no crash | `err_14_raytoaabb_null_out_on_miss` | [x] |
| 15 | `c2AABBtoPoint` | L218 `d0`: `B.x < A.min.x` | returns `0` | `err_15_aabbtopoint_d0` | [x] |
| 16 | `c2AABBtoPoint` | L219 `d1`: `B.y < A.min.y` | returns `0` | `err_16_aabbtopoint_d1` | [x] |
| 17 | `c2AABBtoPoint` | L220 `d2`: `B.x > A.max.x` | returns `0` | `err_17_aabbtopoint_d2` | [x] |
| 18 | `c2AABBtoPoint` | L221 `d3`: `B.y > A.max.y` | returns `0` | `err_18_aabbtopoint_d3` | [x] |
| 19 | `c2CircleToPoint` | L228 `d2 < A.r*A.r` false — point outside/on the circle, incl. `A.r = 0`, `A.r < 0` (`r*r > 0` again!), NaN | returns `0` | `err_19_circletopoint_outside` | [x] |
| 20 | `c2RaytoCapsule` | L291 falls through: `yAe.x*yAp.x >= 0` **and** `min(|yAe.x|,|yAp.x|) >= B.r` | returns `0` **but `*out` has already been overwritten at L243/244** with `{t = 0, n = c2Norm(b-a)}` | `err_20_raytocapsule_fallthrough_writes_out` | [x] |
| 21 | `c2RaytoCapsule` | L272/274/281/283 delegation to `c2RaytoCircle` which itself rejects (rows 1–4) | returns `0`, `*out` = the L243/244 pre-write (cap normal, `t = 0`) | `err_21_raytocapsule_delegated_miss` | [x] |
| 22 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b` → `c2Norm(0,0)` = `(0/0, 0/0)` = NaN,NaN → `M` all NaN, `yAp`/`yBb` NaN | no rejection check triggers on NaN; `capsule_bb.max.y` = NaN; whatever the NaN comparisons yield (C: `c2AABBtoPoint` returns 1 → `return 1` with `*out.n = NaN,NaN`, `t = 0`) | `err_22_raytocapsule_degenerate_ab` | [x] |
| 23 | `c2RaytoCapsule` | `out == NULL` — the C dereferences `out` **unconditionally** at L243 before any check | SIGSEGV. Verified in a child process: C → signal 11; Rust **release** cdylib → signal 11 (identical); Rust **dev** cdylib → signal 6, because `-C debug-assertions=on` detects the null dereference itself (`"null pointer dereference occurred"`) and aborts deliberately. Both are the same rejection; the shipped (release) artifact matches the C exactly | `err_23_raytocapsule_null_out_segv` | [x] |
| 24 | `c2CastRay` | L295 `typeB` has no valid variant (`3`, `4`, `5`, `7`, `255`, `256`, `1000`, `-1`, `-2`, `-1000`, `INT_MAX`, `INT_MIN`, `0x7fffffff`, `-0x80000000`, `0x10000`) → the switch has **no `default`**, so control falls off the end of an `int` function | UB. The disassembly shows the fall-through path jumping straight to `leave; ret` with `eax` never written, so the C "returns" whatever the caller left in `eax`: measured as 5 **different** values in 5 separate processes (ASLR-dependent), e.g. `-1452448196, -583686596, -557959620, -1485650372, -1496000964`. There is no behaviour to reproduce, so the Rust returns a deterministic `0`. The tests assert the whole DEFINED part of the contract: **`*out` is untouched by both**, neither crashes, and a following valid call still works | `err_24_castray_out_of_range_type`, `err_24b_castray_ub_return_is_not_reproducible` | [x] |
| 25 | `c2CastRay` | `B == NULL` with a valid `typeB` → the C dereferences it (`*(c2Circle*)B`, `*(c2AABB*)B`) | SIGSEGV in both (same debug-assertion nuance as row 23). Also covered: a *hit* that writes through `out == NULL` | `err_25_castray_null_shape_segv` | [x] |
| 26 | `c2CastRay` | valid `typeB` but the pointed-to shape rejects (rows 1–4 / 11–13 / 20–21) | same `0`/`*out` as the direct call | `err_26_castray_delegated_miss` | [x] |
| 27 | `c2Div` / `c2Norm` | `b == 0.0f` → `1.0f/0.0f = +inf` → `a * inf` = ±inf or **NaN** (`0 * inf`); `c2Norm` of the zero vector → `(NaN, NaN)` | `(±inf/NaN, ±inf/NaN)`, no error signalled | `err_27_div_by_zero` | [x] |
| 28 | `c2Div` / `c2Norm` | `b == -0.0f` → `1.0f/-0.0f = -inf` (sign matters) | `-inf`-scaled vector | `err_28_div_by_negative_zero` | [x] |
| 29 | `c2Len` / `c2Norm` | overflow: `a.x*a.x` overflows to `+inf` → `sqrtf(+inf) = +inf` → `c2Norm` = `a * (1/inf)` = `a * 0` = `±0` | `+inf` / `(±0,±0)` | `err_29_len_overflow_inf` | [x] |
| 30 | `c2Len` / `c2Dot` | NaN input → NaN out (`sqrtf(NaN)` quiets the NaN, keeps the payload) | NaN (payload: see note) | `err_30_len_nan` | [x] |
| 31 | `spec_ray` | `mp == ray.p` → `c2Norm(0,0)` = NaN → `ray.d` NaN → `ray.t` NaN → `c2RaytoCircle` rejects (row 2) | returns `0`, `*out` untouched | `err_31_spec_ray_degenerate_direction` | [x] |
| 32 | `spec_ray` | `c_r < 0` (negative radius) — `B.r*B.r` is still positive, so the circle behaves like `|r|` | returns whatever the `|r|` circle gives (**not** an error) | `err_32_spec_ray_negative_radius` | [x] |
| 33 | `spec_ray` | `cast == NULL` and the raycast misses → `out` never dereferenced | returns `0`, no crash | `err_33_spec_ray_null_cast_on_miss` | [x] |
| 34 | all `c2*` helpers | ±inf / NaN / denormal / `-0.0` operands (the "one step past a valid range" class for a pure-float API: there is no documented range, so the whole `f32` domain incl. specials is valid input) | bit-identical propagation | `err_34_helpers_special_values` | [x] |
| 35 | `c2Minv`/`c2Maxv`/`c2Absv` | NaN operand — the C uses raw ternaries (`a<b?a:b`, `a<0?-a:a`), **not** `fminf`/`fabsf`: `c2Minv(NaN, x)` returns `x` (second operand) while `c2Minv(x, NaN)` returns `NaN`; `c2Absv(-NaN)` keeps the sign | ternary semantics, asymmetric in NaN | `err_35_minv_maxv_absv_nan_asymmetry` | [x] |

## Note on NaN payloads (the one tolerated difference)

For inputs where **two NaN operands** reach a single multiply/add, IEEE-754
leaves the *payload* of the result unspecified; on x86 SSE the payload comes from
whichever operand the compiler put in the destination register of the
`mulss`/`addss`, i.e. from instruction selection. That is a property of the
compiler, not of the C program:

| comparison (identical NaN corpus, 133 520 comparisons) | non-NaN mismatches | NaN-payload-only differences |
|---|---|---|
| C `-O0` (reference) vs C `-O2` — *the same C source, two builds* | **0** | **2210** |
| C `-O0` (reference) vs the Rust `cdylib` | **0** | 1676 |

The C library therefore disagrees with *itself* on more NaN payloads than the
Rust translation disagrees with the reference (`gcc -O0` emits
`addss %xmm1,%xmm0` with the *second* operand as destination for `c2Add`, while
`-O2` emits a packed `addps %xmm1,%xmm0` with the *first* — opposite payload
priorities). Consequently the harness compares NaN results as "both NaN" and
compares **exact bits for every non-NaN result**; the payload-only difference
count is printed for every row and asserted to be no worse than the C-vs-C
count (`tests/nan_payload_policy.rs`). Setting `SPEC_RAY_STRICT_NAN=1` turns
payload differences into failures for anyone who wants to inspect them.

Across the whole Phase B suite at `SPEC_RAY_N=200000` (**62 929 792**
comparisons) there were **0** hard mismatches and 37 157 NaN-payload-only
differences, every one of them from an input that itself contained a NaN.

## How to re-run

```sh
cd translation && ./verify.sh          # whole matrix: features x profiles x C builds
cargo test --offline --test phase_c_errors -- --test-threads=1   # just Phase C
```
