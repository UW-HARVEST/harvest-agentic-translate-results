#!/usr/bin/env bash
# Sanity-check that the differential suite actually has teeth: inject a series
# of plausible translation bugs into src/lib.rs one at a time and require that
# the suite FAILS for each. Restores the original file afterwards.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

BK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp src/lib.rs "$BK"
restore() { cp "$BK" src/lib.rs; }
trap restore EXIT

# name | perl -pe expression applied to src/lib.rs
MUTANTS=(
  "c2Maxv uses >= instead of >|s/if a\.x > b\.x/if a.x >= b.x/"
  "c2Minv uses f32::min (wrong NaN handling)|s/if a\.x < b\.x \{ a\.x \} else \{ b\.x \}/a.x.min(b.x)/"
  "c2Maxv uses f32::max (wrong NaN handling)|s/if a\.x > b\.x \{ a\.x \} else \{ b\.x \}/a.x.max(b.x)/"
  "c2Clampv argument order swapped|s/c2Maxv\(lo, c2Minv\(a, hi\)\)/c2Minv(hi, c2Maxv(a, lo))/"
  "c2Sub operands swapped|s/a\.x -= b\.x;/a.x = b.x - a.x;/"
  "c2Dot natural (non -O0) operand order|s/let y_prod = mulss\(b\.y, a\.y\);/let y_prod = mulss(a.y, b.y);/"
  "c2Dot add operands swapped|s/addss\(y_prod, x_prod\)/addss(x_prod, y_prod)/"
  "c2CircletoCircle uses <= instead of <|s/\(d2 < r2\) as c_int\n/(d2 <= r2) as c_int\n/"
  "c2AABBtoAABB uses <= instead of <|s/\(B\.max\.x < A\.min\.x\)/(B.max.x <= A.min.x)/"
  "c2AABBtoAABB drops the negation|s/\(\(d0 \| d1 \| d2 \| d3\) == 0\)/((d0 | d1 | d2 | d3) != 0)/"
  "collided AABB/CIRCLE arm forgets the argument swap|s/unsafe \{ \(B as \*const c2Circle\)\.read_unaligned\(\) \},\n                unsafe \{ \(A as \*const c2AABB\)/unsafe { (A as *const c2Circle).read_unaligned() },\n                unsafe { (B as *const c2AABB)/"
  "collided accepts an out-of-range typeA|s/_ => 0,\n    \}\n\}/_ => 1,\n    }\n}/"
  "collided CIRCLE/CIRCLE arm reads the wrong struct|s/C2_TYPE_CIRCLE => c2CircletoCircle\(\n                unsafe \{ \(A as \*const c2Circle\)/C2_TYPE_CIRCLE => c2CircletoCircle(\n                unsafe { (B as *const c2Circle)/"
)

PASS=0; MISS=0
for m in "${MUTANTS[@]}"; do
  name=${m%%|*}
  expr=${m#*|}
  restore
  perl -0777 -pi -e "$expr" src/lib.rs
  if cmp -s "$BK" src/lib.rs; then
    echo "SKIP (mutation did not apply): $name"
    continue
  fi
  if ! timeout 300 cargo build --no-default-features >/dev/null 2>&1; then
    echo "SKIP (mutant does not compile): $name"
    continue
  fi
  if timeout 600 cargo test --no-default-features >/dev/null 2>&1; then
    echo "!! NOT DETECTED: $name"
    MISS=$((MISS + 1))
  else
    echo "detected: $name"
    PASS=$((PASS + 1))
  fi
done

restore
cargo build --no-default-features >/dev/null 2>&1
echo
echo "mutants detected: $PASS   undetected: $MISS"
[ "$MISS" -eq 0 ]
