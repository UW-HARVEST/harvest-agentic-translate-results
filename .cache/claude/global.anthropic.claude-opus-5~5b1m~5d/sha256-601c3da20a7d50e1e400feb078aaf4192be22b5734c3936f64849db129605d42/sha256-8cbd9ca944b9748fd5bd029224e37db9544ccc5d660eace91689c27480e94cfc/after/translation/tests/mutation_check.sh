#!/usr/bin/env bash
# Negative control for the differential suite: prove the tests actually DETECT a
# wrong Rust implementation instead of passing vacuously.
#
# Builds deliberately-broken copies of src/lib.rs as separate cdylibs, points the
# harness at each via RUST_SO, and requires the suite to FAIL for every mutant
# that is not semantically equivalent to the C.
set -uo pipefail
CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE_DIR" || exit 1

OUT=".mutants"
mkdir -p "$OUT"

python3 - <<'PY'
s = open('src/lib.rs').read()
# (id, description, needle, replacement, expect_detected)
muts = [
 (1, "P8 quotient adjustment +1 -> +2",
     "q = t.wrapping_div(nv2).wrapping_add(1);",
     "q = t.wrapping_div(nv2).wrapping_add(2);", True),
 (2, "epilogue sign adjustment flipped",
     "if v2 > 0 { -1 } else { 1 }",
     "if v2 > 0 { 1 } else { -1 }", True),
 (3, "P3 q = 0 -> q = 1",
     "q = 0;\n            r = v1;",
     "q = 1;\n            r = v1;", True),
 (4, "v2 == 0 sentinel 0 -> -1",
     "if v2 == 0 {\n        return 0;\n    }",
     "if v2 == 0 {\n        return -1;\n    }", True),
 (5, "P6 drops the q*v2 term from r",
     "r = v1.wrapping_sub(q.wrapping_mul(v2));",
     "r = v1;", True),
 (6, "replace whole algorithm with std wrapping_div_euclid (EQUIVALENT)",
     'pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {\n    if v2 == 0 {',
     'pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {\n'
     '    if v2 != 0 { return v1.wrapping_div_euclid(v2); }\n    if v2 == 0 {', False),
]
open('.mutants/manifest.txt','w').write(
    "\n".join(f"{i}\t{'DETECT' if d else 'EQUIVALENT'}\t{desc}" for i,desc,_,_,d in muts) + "\n")
for i, desc, a, b, _ in muts:
    t = s.replace(a, b)
    assert t != s, f"mutant {i} needle not found: {desc}"
    open(f'.mutants/m{i}.rs', 'w').write(t)
print(f"generated {len(muts)} mutants")
PY
[ $? -eq 0 ] || exit 1

FAILED=0
while IFS=$'\t' read -r id expect desc; do
  rustc --edition 2021 --crate-type cdylib -O -o "$OUT/libm$id.so" "$OUT/m$id.rs" \
    2>&1 | grep -E '^error' && { echo "!! mutant $id failed to compile"; FAILED=1; continue; }

  RUST_SO="$CRATE_DIR/$OUT/libm$id.so" \
  DIFF_SAMPLES=2000 EXHAUSTIVE_STRIDE=1048573 SWEEP_POINTS=300000 SOAK_SAMPLES=100000 \
    timeout 600 cargo test --offline --release >"$OUT/m$id.log" 2>&1
  nfail=$(grep -cE '^test .* FAILED' "$OUT/m$id.log")

  if [ "$expect" = DETECT ]; then
    if [ "$nfail" -gt 0 ]; then
      echo "PASS  mutant $id  ($nfail tests caught it)  -- $desc"
    else
      echo "!! FAIL mutant $id was NOT detected -- $desc"; FAILED=1
    fi
  else
    if [ "$nfail" -eq 0 ]; then
      echo "PASS  mutant $id  (undetected, as expected: semantically equivalent) -- $desc"
    else
      echo "NOTE  mutant $id  detected ($nfail tests) though expected equivalent -- $desc"
    fi
  fi
done < "$OUT/manifest.txt"

echo
[ "$FAILED" = 0 ] && echo "===== NEGATIVE CONTROL PASSED: suite is not vacuous =====" \
                  || echo "===== NEGATIVE CONTROL FAILED ====="
exit "$FAILED"
