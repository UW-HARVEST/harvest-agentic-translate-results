#!/usr/bin/env bash
# Harness self-validation: deliberately inject a bug into src/lib.rs, confirm the
# differential suite FAILS, then restore. A suite that cannot fail proves nothing.
#
# Each mutation is a plausible translation mistake — the kind a human or an
# automated port would actually make.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp "${TMPDIR:-.}/lib.rs.bak.XXXXXX")
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

# name | sed-style python replacement (old -> new)
run_mutation() {
  local name="$1" old="$2" new="$3"
  restore
  python3 - "$old" "$new" <<'PY'
import sys
old, new = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
if old not in s:
    sys.exit("MUTATION TARGET NOT FOUND: " + old[:70])
s = s.replace(old, new, 1)
open('src/lib.rs', 'w').write(s)
PY
  if [ $? -ne 0 ]; then
    echo "  SKIP  $name (pattern not found)"
    return 1
  fi
  if timeout 600 cargo test --release -q >/dev/null 2>&1; then
    echo "  !!!! NOT CAUGHT: $name  <-- the suite has a blind spot"
    return 2
  else
    echo "  caught: $name"
    return 0
  fi
}

# Some mutations are provably unobservable through the public API. Those are
# asserted to be EQUIVALENT (with the reason), instead of being reported as
# gaps — that keeps the distinction between "the suite is blind" and "the C
# cannot tell either".
run_equivalent() {
  local name="$1" old="$2" new="$3" why="$4"
  restore
  python3 - "$old" "$new" <<'PY2'
import sys
old, new = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
if old not in s:
    sys.exit("TARGET NOT FOUND: " + old[:70])
open('src/lib.rs', 'w').write(s.replace(old, new, 1))
PY2
  if [ $? -ne 0 ]; then
    echo "  SKIP  $name (pattern not found)"
    return 1
  fi
  if timeout 600 cargo test --release -q >/dev/null 2>&1; then
    echo "  equivalent as expected: $name"
    echo "      reason: $why"
    return 0
  else
    echo "  note: $name was CAUGHT (expected it to be unobservable)"
    return 0
  fi
}

echo "=== Mutation testing the differential suite ==="
BLIND=0

run_mutation "c2Add operand order (NaN payload)" \
  'a.x = addss(b.x, a.x); // GCC: reversed' \
  'a.x = addss(a.x, b.x);' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2Dot add operand order (NaN payload)" \
  'addss(t_y, t_x) // GCC: reversed' \
  'addss(t_x, t_y)' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2Div: true division instead of reciprocal-multiply" \
  'c2Mulvs(a, divss(1.0f32, b))' \
  'c2v { x: divss(a.x, b), y: divss(a.y, b) }' || [ $? -eq 1 ] || BLIND=1

run_mutation "c_abs -> f32::abs (breaks -0.0 and NaN sign)" \
  '    if x < 0.0 {
        fneg(x)
    } else {
        x
    }' \
  '    x.abs()' || [ $? -eq 1 ] || BLIND=1

run_mutation "c_min -> f32::min (breaks NaN asymmetry)" \
  '    if a < b {
        a
    } else {
        b
    }' \
  '    a.min(b)' || [ $? -eq 1 ] || BLIND=1

run_mutation "c_max -> f32::max (breaks NaN asymmetry)" \
  '    if a > b {
        a
    } else {
        b
    }' \
  '    a.max(b)' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2CastRay accepts an out-of-range enum value" \
  '        _ => {}' \
  '        _ => return c2RaytoCircle(A, (B as *const c2Circle).read_unaligned(), out),' \
  || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoPoly: hi < lo becomes hi <= lo" \
  'if hi < lo {' \
  'if hi <= lo {' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoPoly: index sentinel ~0 -> 0" \
  'let mut index: c_int = !0;' \
  'let mut index: c_int = 0;' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoPoly: clamp count to 8 (a 'safety fix' the C does not do)" \
  'while i < addr_of!((*B).count).read_unaligned() {' \
  'while i < addr_of!((*B).count).read_unaligned().min(8) {' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoPoly: reject NULL bx instead of using identity" \
  '    let bx = if !bx_ptr.is_null() {
        bx_ptr.read_unaligned()
    } else {
        c2xIdentity()
    };' \
  '    if bx_ptr.is_null() {
        return 0;
    }
    let bx = bx_ptr.read_unaligned();' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoCircle: t <= A.t becomes t < A.t" \
  'if t >= 0.0 && t <= A.t {' \
  'if t >= 0.0 && t < A.t {' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoCircle: disc < 0 becomes disc <= 0" \
  'if disc < 0.0 {' \
  'if disc <= 0.0 {' || [ $? -eq 1 ] || BLIND=1

# NOTE: only the t3 (`else`) branch can be reached with a NaN `t_k`, because
# selecting t0/t1/t2 requires all four to be mutually ordered. So the t0/t1/t2
# operand orders are provably unobservable and are NOT mutated here; see the
# equivalence notes in src/lib.rs.
run_mutation "c2RaytoAABB: out->t operand order, t3 branch (NaN payload)" \
  'addr_of_mut!((*out).t).write_unaligned(mulss(A.t, t3)); // GCC: reversed' \
  'addr_of_mut!((*out).t).write_unaligned(mulss(t3, A.t));' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoAABB: SAT subtraction operands swapped" \
  '    let d = subss(
        c_abs(c2Dot(n, c2Sub(p0, center_of_b_box))),
        c2Dot(abs_n, half_extents),
    );' \
  '    let d = subss(
        c2Dot(abs_n, half_extents),
        c_abs(c2Dot(n, c2Sub(p0, center_of_b_box))),
    );' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2MulmvT: first mul operand order (NaN payload)" \
  'let cx_1 = mulss(a.x.x, b.x);' \
  'let cx_1 = mulss(b.x, a.x.x);' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoAABB: swap the (0,-1) and (0,1) normals" \
  'addr_of_mut!((*out).n).write_unaligned(c2V(0.0, -1.0));' \
  'addr_of_mut!((*out).n).write_unaligned(c2V(0.0, 1.0));' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoCapsule: drop the pre-write of *out" \
  '    addr_of_mut!((*out).n).write_unaligned(c2Norm(cap_n));
    addr_of_mut!((*out).t).write_unaligned(0.0);' \
  '    // pre-write removed' || [ $? -eq 1 ] || BLIND=1

# NOTE: the *order* of this addss is provably unobservable (reaching the
# side-plane branch forces yAp.x/yAe.x non-NaN, which forces yAp.y non-NaN, and
# addition of non-NaNs is commutative). A 20M-case search over an alphabet built
# specifically to make two distinct NaNs meet here found no witness. So mutate
# the non-commutative subtraction inside it, which IS observable.
run_mutation "c2RaytoCapsule: y interpolation subtraction reversed" \
  'let y = addss(mulss(subss(yAe.y, yAp.y), t), yAp.y); // GCC: reversed' \
  'let y = addss(mulss(subss(yAp.y, yAe.y), t), yAp.y);' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2RaytoCapsule: c > 0 becomes c >= 0" \
  'if c > 0.0 { M.x } else { c2Skew(M.y) }' \
  'if c >= 0.0 { M.x } else { c2Skew(M.y) }' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2CircleToPoint: strict < becomes <=" \
  '(d2 < mulss(A.r, A.r)) as c_int' \
  '(d2 <= mulss(A.r, A.r)) as c_int' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2AABBtoPoint: one axis uses >= instead of >" \
  'let d2 = (B.x > A.max.x) as c_int;' \
  'let d2 = (B.x >= A.max.x) as c_int;' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2Skew: forget the negation" \
  'b.x = fneg(a.y);' \
  'b.x = a.y;' || [ $? -eq 1 ] || BLIND=1

run_mutation "c2MulrvT: -a.s*b.x parsed as -(a.s*b.x) ... plus a sign slip" \
  'let arg1 = addss(mulss(a.c, b.x), mulss(b.y, a.s)); // GCC: 2nd mul reversed' \
  'let arg1 = subss(mulss(a.c, b.x), mulss(b.y, a.s));' || [ $? -eq 1 ] || BLIND=1

# NOTE: `f32::sqrt` already lowers to SQRTSS and reproduces its NaN handling
# exactly (verified: sqrt(-1.0) == 0xFFC00000, sqrt(sNaN) is quieted), and the
# only caller passes `c2Dot(a,a)`, which can never be negative. So dropping the
# guard clauses is a semantic no-op. Mutate an observable property instead: the
# sign bit of a propagated NaN.
run_mutation "sqrtss: normalise the NaN sign (as fabs would)" \
  '    if x.is_nan() {
        quiet(x)
    } else if x < 0.0 {' \
  '    if x.is_nan() {
        quiet(f32::from_bits(x.to_bits() & 0x7FFF_FFFF))
    } else if x < 0.0 {' || [ $? -eq 1 ] || BLIND=1

# NOTE: in the hard-coded scenario BOTH rays miss, so permuting them is
# unobservable through the public API (the C behaves identically). Mutate the
# ray ORIGIN instead, which turns a miss into a hit.
run_mutation "poly_ray: perturb the hard-coded ray origin" \
  'x: -3.869416f32,
            y: 13.0693407f32,' \
  'x: -3.869416f32,
            y: 0.0f32,' || [ $? -eq 1 ] || BLIND=1

run_equivalent "poly_ray: change the hard-coded polygon vertex" \
  'p.verts[0] = c2V(0.875f32, -11.5f32);' \
  'p.verts[0] = c2V(8.875f32, -11.5f32);' \
  "both hard-coded rays exit c2RaytoPoly at i==1 via \`den==0 && num<0\` on the
              y-plane, which does not read verts[0]; the C therefore cannot
              distinguish this change either"

run_equivalent "poly_ray: drop the << 1 on the second hit" \
  '        cast2,
    ) << 1;' \
  '        cast2,
    );' \
  "the second ray MISSES in the fixed scenario, so the shifted value is
              0 << 1 == 0; poly_ray takes no arguments, so no caller can ever
              observe the shift"

run_mutation "quiet(): do not quiet signalling NaNs" \
  'f32::from_bits(x.to_bits() | 0x0040_0000)' \
  'x' || [ $? -eq 1 ] || BLIND=1

restore
echo
echo "=== Confirming the restored source still passes ==="
if timeout 600 cargo test --release -q >/dev/null 2>&1; then
  echo "  restored source: PASS"
else
  echo "  restored source: FAIL  <-- restore went wrong!"
  BLIND=1
fi

echo
if [ "$BLIND" -eq 0 ]; then
  echo "############ every observable mutation was caught ############"
else
  echo "############ BLIND SPOTS FOUND ############"
fi
exit "$BLIND"
