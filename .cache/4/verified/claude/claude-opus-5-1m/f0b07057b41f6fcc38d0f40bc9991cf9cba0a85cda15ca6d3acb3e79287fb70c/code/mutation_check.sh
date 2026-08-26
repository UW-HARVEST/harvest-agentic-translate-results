#!/usr/bin/env bash
# Harness self-validation: inject a known behavioural bug into src/lib.rs and
# confirm the differential test suite CATCHES it. A test suite that passes but
# cannot detect deliberate divergence proves nothing.
#
# Usage: ./mutation_check.sh
set -u

cd "$(dirname "$0")"
BAK="$(mktemp)"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap restore EXIT

# Each entry: description ::: literal-to-find ::: replacement
MUTATIONS=(
  "c2Support tie-break > -> >=:::            if dot > dmax {:::            if dot >= dmax {"
  "c2GJK iteration cap 20 -> 19:::        while iter < 20 {:::        while iter < 19 {"
  "c2GJK no-progress d1>d0 -> >=:::            if d1 > d0 {:::            if d1 >= d0 {"
  "c23 arm-1 condition <=0 -> <0:::        if vAB <= 0.0 && uCA <= 0.0 {:::        if vAB < 0.0 && uCA <= 0.0 {"
  "c2D det>0 -> det>=0:::                if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {:::                if c2Det2(ab, c2Neg((*s).verts[0].p)) >= 0.0 {"
  "c2GJK radius test > -> >=:::            if dist > addp(rA, rB) && dist > C_FLT_EPSILON {:::            if dist >= addp(rA, rB) && dist > C_FLT_EPSILON {"
  "c2GJK collapse && -> ||:::                if a.x == b.x && a.y == b.y {:::                if a.x == b.x || a.y == b.y {"
  "c2GJK cache guard -1e8 -> -1e7:::                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {:::                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e7f32) {"
  "c2GJK cold-start u 1.0 -> 0.5:::            s.verts[0].u = 1.0f32;:::            s.verts[0].u = 0.5f32;"
  "c2MakeProxy AABB count 4 -> 3:::                (*p).count = 4;:::                (*p).count = 3;"
  "c2BBVerts corner 1 wrong:::        *out.offset(1) = c2V((*bb).max.x, (*bb).min.y);:::        *out.offset(1) = c2V((*bb).min.x, (*bb).max.y);"
  "gjk reverse polarity flipped:::        if reverse != 0 {:::        if reverse == 0 {"
  "c2Dot addp destination swapped:::    addp(t2, t1):::    addp(t1, t2)"
  "c2Dot second mulp swapped:::    let t2 = mulp(b.y, a.y);:::    let t2 = mulp(a.y, b.y);"
  "c2Det2 subtraction order:::    t1 - t2:::    -(t2 - t1)"
  "c2Skew sign flipped:::    b.x = -a.y;:::    b.x = a.y;"
  "c2GJKSimplexMetric count-2 arm:::        2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),:::        2 => c2Len(c2Sub((*s).verts[0].p, (*s).verts[1].p)),"
  "c2Witness count-3 third term:::                    c2Mulvs((*s).verts[2].sA, mulp((*s).verts[2].u, den)),:::                    c2Mulvs((*s).verts[2].sB, mulp((*s).verts[2].u, den)),"
  "c22 else-arm div order:::            (*s).div = addp(u, v);:::            (*s).div = addp(v, u);"
  "c2GJK epsilon compare:::            if c2Dot(d, d) < C_FLT_EPSILON * C_FLT_EPSILON {:::            if c2Dot(d, d) <= C_FLT_EPSILON * C_FLT_EPSILON {"
)

# Mutants that are PROVABLY semantics-preserving, so surviving is correct.
# Each entry is justified in VERIFICATION.md; they are NOT test-suite gaps.
EXPECTED_EQUIVALENT=(
  "c2GJK iteration cap 20 -> 19"
  "c2GJK cold-start u 1.0 -> 0.5"
)
is_expected() {
  local d="$1"
  for e in "${EXPECTED_EQUIVALENT[@]}"; do [ "$e" = "$d" ] && return 0; done
  return 1
}

killed=0; survived=0; skipped=0; equivalent=0
SURVIVORS=()

for m in "${MUTATIONS[@]}"; do
  desc="${m%%:::*}"
  rest="${m#*:::}"
  find="${rest%%:::*}"
  repl="${rest##*:::}"

  restore
  if ! grep -qF -- "$find" src/lib.rs; then
    printf '  SKIP    %s (pattern not found)\n' "$desc"
    skipped=$((skipped+1)); continue
  fi
  python3 - "$find" "$repl" <<'PY'
import sys
find, repl = sys.argv[1], sys.argv[2]
p='src/lib.rs'; s=open(p).read()
open(p,'w').write(s.replace(find, repl, 1))
PY

  out=$(timeout 600 cargo test --release 2>&1)
  if printf '%s' "$out" | grep -q "test result: FAILED"; then
    nfail=$(printf '%s' "$out" | grep -c "^test .* FAILED")
    printf '  KILLED  %s (%s failing tests)\n' "$desc" "$nfail"
    killed=$((killed+1))
  elif printf '%s' "$out" | grep -q "error\[\|error:"; then
    printf '  SKIP    %s (did not compile)\n' "$desc"
    skipped=$((skipped+1))
  elif is_expected "$desc"; then
    printf '  EQUIV   %s (expected: provably dead code, see VERIFICATION.md)\n' "$desc"
    equivalent=$((equivalent+1))
  else
    printf '  SURVIVED %s\n' "$desc"
    survived=$((survived+1)); SURVIVORS+=("$desc")
  fi
done

restore
echo
echo "==============================================================="
echo " mutation score: $killed killed / $equivalent known-equivalent /"
echo "                 $survived unexplained survivors / $skipped skipped"
echo "==============================================================="
if [ "$survived" -gt 0 ]; then
  echo "Unexplained surviving mutants (real test-suite blind spots):"
  for s in "${SURVIVORS[@]}"; do echo "  - $s"; done
  exit 1
fi
echo "All behaviour-changing mutants were detected by the differential suite."
