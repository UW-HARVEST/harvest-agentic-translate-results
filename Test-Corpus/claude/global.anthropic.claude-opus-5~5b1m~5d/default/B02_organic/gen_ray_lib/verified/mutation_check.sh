#!/usr/bin/env bash
# Verification-of-the-verification.
#
# Injects each known-wrong variant into src/lib.rs one at a time and confirms the
# differential suite CATCHES it. A mutation that survives is either a real blind
# spot in the test suite (a failure) or a semantically EQUIVALENT mutant — one
# whose difference is unobservable because the code path that would expose it is
# unreachable. Equivalent mutants are listed explicitly, each with the reason,
# so they cannot be used to hide a genuine gap.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here" || exit 1

LOGDIR="${TMPDIR:-$here/target}/verify-logs"
mkdir -p "$LOGDIR" || LOGDIR="$here/target"

SRC=src/lib.rs
BAK="$LOGDIR/lib.rs.orig"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

# ---------------------------------------------------------------------------
# Mutations that are EXPECTED to survive, because they are provably equivalent.
#
# All four SSE arithmetic ops return the *destination* operand when BOTH operands
# are NaN, but return the single NaN's payload (order-independently) when only one
# is. So swapping operands is observable only where both can be NaN at once.
#
#  * "aabb out.t t0/t1/t2":  the branch that uses `t0` requires
#      `t0 >= t1 && t0 >= t2 && t0 >= t3` — an ordered comparison that is false
#      for NaN. So `t0` (likewise t1, t2) is never NaN in its own branch, and
#      only the final `else` branch (t3) can multiply two NaNs. The t3 branch IS
#      covered (it kills its mutant), which proves the site is reachable.
#  * "aabb hit_i as f32 * t_i":  `hit_i as f32` is exactly 0.0 or 1.0, never NaN.
#  * "capsule out.t = A.t * t":  reaching the side-wall branch needs
#      `abs(yAp.x) >= B.r` (ordered ⇒ yAp.x not NaN) plus entry into the outer
#      `if`. If `A.t` is NaN then `yAe.x` is NaN, so `yAe.x*yAp.x < 0` is false and
#      `min_ternary(abs(yAe.x), abs(yAp.x))` collapses to `abs(yAp.x)`, making the
#      outer test identical to the inner `abs(yAp.x) < B.r` — which then routes to
#      the end-cap delegate, not to the multiply. So A.t and t are never both NaN.
#  * "capsule y = prod + yAp.y":  `yAp.y` can only be NaN if `yAp.x` is too
#      (both come from the same `c2MulmvT` of the same input vector, and the only
#      NaN-producing path, `inf*0`, needs `M.y.y == 0`, which forces
#      `M.x.y == ∓1` and hence an infinite — not NaN — `yAp.x`; a NaN `yAp.x`
#      makes the ordered guards above fail). So the two addends are never both NaN.
#  * "capsule c sign test >= vs >":  the two differ only for `yAp.x == +/-0.0`
#      (for NaN both are false). Reaching `c` needs `abs(yAp.x) >= B.r`, i.e.
#      `0 >= B.r`; but entering the outer `if` with `yAp.x == +/-0.0` needs
#      `min_ternary(abs(yAe.x), +/-0.0) < B.r`, and `min_ternary(a, +/-0.0)` always
#      returns `+/-0.0` (because `abs_ternary` never yields a value ordered below
#      zero), so it needs `B.r > 0`. `0 >= B.r` and `B.r > 0` are contradictory,
#      and `yAe.x*yAp.x < 0` is false when `yAp.x` is zero. Unreachable.
#  * "c2Div reciprocal": `fdiv(1.0, b)` and `1.0f32 / b` are the same `divss`.
#  * "c2Len abs(dot)": `dot(a,a) = a.x*a.x + a.y*a.y` is never negative and never
#      `-0.0`, and `abs_ternary` is the identity on NaN, so it is a no-op.
# ---------------------------------------------------------------------------
declare -A EXPECTED_SURVIVOR=(
  ["c2RaytoAABB: out->t operands swapped in the t0 branch"]=1
  ["c2RaytoAABB: out->t operands swapped in the t1 branch"]=1
  ["c2RaytoAABB: out->t operands swapped in the t2 branch"]=1
  ["c2RaytoAABB: (float)hit3 * t3 operands swapped"]=1
  ["c2RaytoCapsule: out->t operands swapped"]=1
  ["c2RaytoCapsule: y addss operands swapped"]=1
  ["c2RaytoCapsule: c sign test >= instead of >"]=1
  ["c2Div: reciprocal spelled as a plain / (same divss)"]=1
  ["c2Len: abs() around dot(a,a) (a no-op)"]=1
)

# Each mutation is "description|||from|||to"
mutations=(
# ---- operand order (the class of bug this translation is most exposed to) ----
"c2Dot: addss destination = x-product instead of y-product|||    fadd(y_prod, x_prod)|||    fadd(x_prod, y_prod)"
"c2Dot: y mulss destination = a.y instead of b.y|||    let y_prod = fmul(b.y, a.y);|||    let y_prod = fmul(a.y, b.y);"
"c2Add: addss destination = a instead of b|||    a.x = fadd(b.x, a.x);
    a.y = fadd(b.y, a.y);|||    a.x = fadd(a.x, b.x);
    a.y = fadd(a.y, b.y);"
"c2Sub: subss operands swapped|||    a.x = fsub(a.x, b.x);|||    a.x = fsub(b.x, a.x);"
"c2Mulvs: y mulss destination = b instead of a.y|||    a.y = fmul(a.y, b);|||    a.y = fmul(b, a.y);"
"c2Mulvs: x mulss destination = b instead of a.x|||    a.x = fmul(a.x, b);|||    a.x = fmul(b, a.x);"
"c2MulmvT: row-0 addss destination swapped|||    c.x = fadd(x1, x0);|||    c.x = fadd(x0, x1);"
"c2MulmvT: row-1 addss destination swapped|||    c.y = fadd(y1, y0);|||    c.y = fadd(y0, y1);"
"c2MulmvT: row-0 second mulss destination swapped|||    let x1 = fmul(b.y, a.x.y);|||    let x1 = fmul(a.x.y, b.y);"
"c2MulmvT: row-1 second mulss destination swapped|||    let y1 = fmul(b.y, a.y.y);|||    let y1 = fmul(a.y.y, b.y);"
"c2RaytoCircle: disc subss operands swapped|||    let disc = fsub(fmul(b, b), c);|||    let disc = fsub(c, fmul(b, b));"
"c2RaytoCircle: c subss operands swapped|||    let c = fsub(dot(m, m), fmul(B.r, B.r));|||    let c = fsub(fmul(B.r, B.r), dot(m, m));"
"c2RaytoCircle: t subss operands swapped|||    let t = fsub(-b, fsqrt(disc));|||    let t = fsub(fsqrt(disc), -b);"
"c2RaytoAABB: out->t operands swapped in the t0 branch|||                (*out).t = fmul(A.t, t0);|||                (*out).t = fmul(t0, A.t);"
"c2RaytoAABB: out->t operands swapped in the t1 branch|||                (*out).t = fmul(A.t, t1);|||                (*out).t = fmul(t1, A.t);"
"c2RaytoAABB: out->t operands swapped in the t2 branch|||                (*out).t = fmul(A.t, t2);|||                (*out).t = fmul(t2, A.t);"
"c2RaytoAABB: out->t operands swapped in the t3/else branch|||                (*out).t = fmul(A.t, t3);|||                (*out).t = fmul(t3, A.t);"
"c2RaytoAABB: (float)hit3 * t3 operands swapped|||    t3 = fmul(hit3 as f32, t3);|||    t3 = fmul(t3, hit3 as f32);"
"c2RaytoAABB: SAT subss operands swapped|||    let d = fsub(
        abs_ternary(dot(n, sub(p0, center_of_b_box))),
        dot(abs_n, half_extents),
    );|||    let d = fsub(
        dot(abs_n, half_extents),
        abs_ternary(dot(n, sub(p0, center_of_b_box))),
    );"
"c2RaytoCapsule: out->t operands swapped|||                    (*out).t = fmul(A.t, t);|||                    (*out).t = fmul(t, A.t);"
"c2RaytoCapsule: y addss operands swapped|||    let y = fadd(fmul(fsub(yAe.y, yAp.y), t), yAp.y);|||    let y = fadd(yAp.y, fmul(fsub(yAe.y, yAp.y), t));"
"c2RaytoCapsule: t divss operands swapped|||    let t = fdiv(fsub(c, yAp.x), d);|||    let t = fdiv(d, fsub(c, yAp.x));"
"c2RaytoCapsule: d subss operands swapped|||    let d = fsub(yAe.x, yAp.x);|||    let d = fsub(yAp.x, yAe.x);"
"c2SignedDistPointToPlane: subss operands swapped|||    fsub(fmul(p, n), fmul(d, n))|||    fsub(fmul(d, n), fmul(p, n))"
"c2RayToPlane: d subss operands swapped|||        let d = fsub(da, db);|||        let d = fsub(db, da);"
"gen_ray: ray.t subss operands swapped|||    ray.t = fsub(dot(mp, ray.d), dot(ray.p, ray.d));|||    ray.t = fsub(dot(ray.p, ray.d), dot(mp, ray.d));"
# ---- the C's non-libm ternary idioms ----
"c2Absv: use f32::abs instead of the C ternary|||    if a < 0.0 { -a } else { a }|||    a.abs()"
"c2Minv: use f32::min instead of the C ternary|||    if a < b { a } else { b }|||    a.min(b)"
"c2Maxv: use f32::max instead of the C ternary|||    if a > b { a } else { b }|||    a.max(b)"
"c2Div: reciprocal spelled as a plain / (same divss)|||    mulvs(a, fdiv(1.0f32, b))|||    mulvs(a, 1.0f32 / b)"
"c2Div: divide each lane directly (drops the reciprocal quirk)|||    mulvs(a, fdiv(1.0f32, b))|||    c2v { x: fdiv(a.x, b), y: fdiv(a.y, b) }"
"c2Len: abs() around dot(a,a) (a no-op)|||    fsqrt(dot(a, a))|||    fsqrt(abs_ternary(dot(a, a)))"
# ---- comparison boundaries ----
"c2RaytoCircle: disc guard <= instead of <|||    if disc < 0.0 {|||    if disc <= 0.0 {"
"c2RaytoCircle: t guard > 0 instead of >= 0|||    if t >= 0.0 && t <= A.t {|||    if t > 0.0 && t <= A.t {"
"c2RaytoCircle: t guard < A.t instead of <= A.t|||    if t >= 0.0 && t <= A.t {|||    if t >= 0.0 && t < A.t {"
"c2RaytoAABB: SAT guard >= instead of >|||    if d > 0.0 {|||    if d >= 0.0 {"
"c2RaytoAABB: hit_i guard < instead of <=|||    let hit0 = (t0 <= 1.0f32) as c_int;|||    let hit0 = (t0 < 1.0f32) as c_int;"
"c2RayToPlane: zero-denominator guard removed|||        if d != 0.0 { fdiv(da, d) } else { 0.0 }|||        fdiv(da, d)"
"c2RayToPlane: da guard <= instead of <|||    if da < 0.0 {|||    if da <= 0.0 {"
"c2RayToPlane: same-side guard >= instead of >|||    } else if fmul(da, db) > 0.0 {|||    } else if fmul(da, db) >= 0.0 {"
"c2CircleToPoint: <= instead of <|||    (d2 < fmul(A.r, A.r)) as c_int|||    (d2 <= fmul(A.r, A.r)) as c_int"
"c2RaytoCapsule: y <= 0 becomes y < 0|||            if y <= 0.0 {|||            if y < 0.0 {"
"c2RaytoCapsule: y >= yBb.y becomes y > yBb.y|||            if y >= yBb.y {|||            if y > yBb.y {"
"c2RaytoCapsule: c sign test >= instead of >|||            let c = if yAp.x > 0.0 { B.r } else { -B.r };|||            let c = if yAp.x >= 0.0 { B.r } else { -B.r };"
"c2RaytoCapsule: normal picks skew(M.y) instead of M.x|||                    (*out).n = if c > 0.0 { M.x } else { skew(M.y) };|||                    (*out).n = if c > 0.0 { skew(M.y) } else { M.x };"
"c2RaytoCapsule: yAp.y sign test flipped|||            if yAp.y < 0.0 {|||            if yAp.y > 0.0 {"
# ---- structure / bookkeeping ----
"c2AABBtoAABB: one separating axis dropped|||    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// \`c2AABBtoPoint\`|||    ((d0 | d1 | d2) == 0) as c_int
}

/// \`c2AABBtoPoint\`"
"c2AABBtoPoint: one separating axis dropped|||    let d3 = (B.y > A.max.y) as c_int;|||    let d3 = 0 as c_int;"
"c2Skew: negate the wrong lane|||    b.x = -a.y;
    b.y = a.x;|||    b.x = a.y;
    b.y = -a.x;"
"c2CCW90: negate the wrong lane|||    b.x = a.y;
    b.y = -a.x;|||    b.x = -a.y;
    b.y = a.x;"
"c2RaytoCapsule: capsule_bb.min uses +B.r|||    capsule_bb.min = c2v_new(-B.r, 0.0);|||    capsule_bb.min = c2v_new(B.r, 0.0);"
"c2RaytoCapsule: out->n pre-set skipped|||        (*out).n = norm(cap_n);
        (*out).t = 0.0;|||        (*out).t = 0.0;"
"c2CastRay: return 0 for out-of-range typeB instead of preserving %eax|||        \"cmp esi, 2\",
        \"ja 2f\",|||        \"cmp esi, 2\",
        \"ja 8f\",
        \"jmp {dispatch}\",
        \"8:\",
        \"xor eax, eax\",
        \"ret\",
        \"ja 2f\","
"c2CastRay: signed range check (accepts negative typeB)|||        \"cmp esi, 2\",
        \"ja 2f\",|||        \"cmp esi, 2\",
        \"jg 2f\","
"gen_ray: capsule result shifted by 3 instead of 1|||        } << 1,|||        } << 3,"
"gen_ray: aabb result shifted by 1 instead of 2|||C2_TYPE_AABB, cast3) } << 2,|||C2_TYPE_AABB, cast3) } << 1,"
"gen_ray: cast1/cast2 out-pointers swapped|||C2_TYPE_CIRCLE, cast1)|||C2_TYPE_CIRCLE, cast2)"
"gen_ray: ray.d not normalised|||    ray.d = norm(sub(mp, ray.p));|||    ray.d = sub(mp, ray.p);"
)

printf '%-74s %s\n' "MUTATION" "RESULT"
printf '%.0s-' {1..100}; echo

survivors=0
killed=0
skipped=0
bad_survivors=()

for m in "${mutations[@]}"; do
    desc="${m%%|||*}"
    rest="${m#*|||}"
    from="${rest%%|||*}"
    to="${rest#*|||}"

    restore
    if ! python3 - "$SRC" "$from" "$to" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if s.count(frm) != 1:
    sys.exit(2)
open(path, 'w').write(s.replace(frm, to, 1))
PY
    then
        printf '%-74s %s\n' "$desc" "SKIP (pattern not unique/found)"
        skipped=$((skipped + 1))
        continue
    fi

    if ! cargo build --offline --release >"$LOGDIR/mut_build.log" 2>&1; then
        printf '%-74s %s\n' "$desc" "KILLED (compile error)"
        killed=$((killed + 1))
        continue
    fi

    if cargo test --offline --release >"$LOGDIR/mut_test.log" 2>&1; then
        if [[ -n "${EXPECTED_SURVIVOR[$desc]:-}" ]]; then
            printf '%-74s %s\n' "$desc" "survived (EXPECTED: equivalent mutant)"
        else
            printf '%-74s %s\n' "$desc" "*** SURVIVED — BLIND SPOT ***"
            bad_survivors+=("$desc")
        fi
        survivors=$((survivors + 1))
    else
        first=$(grep -oE '^---- [a-z0-9_]+' "$LOGDIR/mut_test.log" | head -n1 | sed 's/---- //')
        printf '%-74s %s\n' "$desc" "killed by ${first:-a test}"
        killed=$((killed + 1))
        if [[ -n "${EXPECTED_SURVIVOR[$desc]:-}" ]]; then
            echo "    NOTE: this was listed as an equivalent mutant but was killed —"
            echo "          the allowlist entry can be removed."
        fi
    fi
done

restore
cargo build --offline --release >/dev/null 2>&1

echo
echo "mutations: $((killed + survivors + skipped))   killed: $killed   survived: $survivors   skipped: $skipped"
if [[ ${#bad_survivors[@]} -ne 0 ]]; then
    echo
    echo "FAIL: ${#bad_survivors[@]} unexplained survivor(s) — the suite has a blind spot:"
    printf '  - %s\n' "${bad_survivors[@]}"
    exit 1
fi
if [[ $skipped -ne 0 ]]; then
    echo "FAIL: $skipped mutation pattern(s) did not apply — the script is stale."
    exit 1
fi
echo "PASS: every observable injected divergence was detected; all survivors are"
echo "      documented equivalent mutants."
