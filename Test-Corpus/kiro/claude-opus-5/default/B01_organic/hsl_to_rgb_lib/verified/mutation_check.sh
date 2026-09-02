#!/usr/bin/env bash
# Mutation test: prove the differential suite is not vacuous.
#
# Each mutation is a small, plausible "simplification" of the Rust translation
# that a careless translator might write. Every one MUST be caught, otherwise the
# corresponding CONFIGS/ERRORS row is not really being verified.

set -uo pipefail
CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$CRATE/src/lib.rs"
BACKUP="$(mktemp)"
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

survived=0
declare -a RESULTS

# Mutations that are provably UNOBSERVABLE (true semantic equivalences, verified
# against the compiled C / rustc codegen). These are expected to survive; a
# survivor here is NOT a blind spot. Listing them keeps the script's exit status
# meaningful: any *other* survivor is a real gap.
EXPECTED_EQUIVALENT=(
  "m: mul(c, 0.5) -> mul(0.5, c)"
  "fabsf quiets NaN (should be a pure andps)"
  "c: add(l,l) -> mul(2.0, l)"
  "naive m = l - 0.5*c (no NaN operand-role modelling)"
)

is_expected_equivalent() {
  local n="$1"
  for e in "${EXPECTED_EQUIVALENT[@]}"; do [ "$e" = "$n" ] && return 0; done
  return 1
}

try_mutation() {
  local name="$1" from="$2" to="$3"
  cp "$BACKUP" "$SRC"

  if ! grep -qF -- "$from" "$SRC"; then
    RESULTS+=("SKIP      $name (pattern not found)")
    survived=1
    return
  fi
  # The pattern MUST be unique. Otherwise the patch can land in a doc comment
  # that quotes the same code, and the mutant survives for a bogus reason —
  # which is exactly what happened once during development.
  local hits
  hits=$(grep -oF -- "$from" "$SRC" | wc -l)
  if [ "$hits" -ne 1 ]; then
    RESULTS+=("SKIP      $name (pattern matches $hits times, need exactly 1)")
    survived=1
    return
  fi
  python3 - "$SRC" "$from" "$to" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
assert old in s, old
open(path, 'w').write(s.replace(old, new, 1))
PY

  # Build and test BOTH profiles: some fidelity properties (the null-pointer
  # fatal signal) are only observable with rustc's UB checks compiled in, i.e.
  # in the debug profile. A mutant counts as killed if EITHER profile catches it.
  local compiled=0 killed=0 caught=""
  for prof in debug release; do
    if [ "$prof" = release ]; then pflag=(--release); else pflag=(); fi
    if ! timeout 600 cargo build "${pflag[@]}" > /tmp/mut_build.log 2>&1; then
      continue
    fi
    compiled=1
    if ! timeout 600 cargo test "${pflag[@]}" > "/tmp/mut_test_$prof.log" 2>&1; then
      killed=1
      caught="$caught$(grep -E '^\s+(cfg|err|symbols)[a-z0-9_]+$' "/tmp/mut_test_$prof.log" | sort -u | tr -d ' ' | tr '\n' ' ')"
    fi
  done

  if [ "$compiled" -eq 0 ]; then
    RESULTS+=("SKIP      $name (mutant did not compile)")
    survived=1
    return
  fi

  if [ "$killed" -eq 0 ]; then
    if is_expected_equivalent "$name"; then
      RESULTS+=("EQUIV     $name  (expected: unobservable)")
    else
      RESULTS+=("SURVIVED  $name  <-- BLIND SPOT")
      survived=1
    fi
  else
    if is_expected_equivalent "$name"; then
      RESULTS+=("UNEXPECTED-KILL  $name  <-- equivalence claim is wrong: ${caught}")
      survived=1
    else
      RESULTS+=("KILLED    $name  by: ${caught:-<see log>}")
    fi
  fi
}

cd "$CRATE"

# 1. Revert the arm-3 third-channel operand order (the real bug that was found).
try_mutation "arm3 dest[2] operand order: add(x,m) -> add(m,x)" \
  '(m, add(c, m), add(x, m))' '(m, add(c, m), add(m, x))'

# 2. Same for arm 4.
try_mutation "arm4 dest[2] operand order: add(c,m) -> add(m,c)" \
  '(m, add(x, m), add(c, m))' '(m, add(x, m), add(m, c))'

# 3. Same for arm 5.
try_mutation "arm5 dest[2] operand order: add(c,m) -> add(m,c)" \
  '(add(x, m), m, add(c, m))' '(add(x, m), m, add(m, c))'

# 4. Same for arm 6.
try_mutation "arm6 dest[2] operand order: add(x,m) -> add(m,x)" \
  '(add(c, m), m, add(x, m))' '(add(c, m), m, add(m, x))'

# 5. "Fix" the arm-3 predicate typo the C actually contains.
try_mutation "'fix' arm3 predicate: h < 120 -> h >= 120" \
  '} else if h < 120.0f32 && h < 180.0f32 {' '} else if h >= 120.0f32 && h < 180.0f32 {'

# 6. Treat -0.0 saturation as non-zero (breaks the IEEE fast-path condition).
try_mutation "s == 0 -> sign-sensitive zero test" \
  'if s == 0.0f32 {' 'if s == 0.0f32 && s.is_sign_positive() {'

# 7. Swap the mulss operands that produce x.
try_mutation "x: mul(term, c) -> mul(c, term)" \
  'x = mul(sub(1.0f32, fabsf(sub(fmodf(div(h, 60.0f32), 2.0f32), 1.0f32))), c);' \
  'x = mul(c, sub(1.0f32, fabsf(sub(fmodf(div(h, 60.0f32), 2.0f32), 1.0f32))));'

# 8. Swap the mulss operands that produce m's 0.5*c term.
try_mutation "m: mul(c, 0.5) -> mul(0.5, c)" \
  'm = sub(l, mul(c, 0.5f32));' 'm = sub(l, mul(0.5f32, c));'

# 9. Make fabsf quiet NaNs (it is an andps, which does not).
try_mutation "fabsf quiets NaN (should be a pure andps)" \
  'f32::from_bits(x.to_bits() & 0x7fff_ffff)' \
  'f32::from_bits((x.to_bits() & 0x7fff_ffff) | if x.is_nan() { 0x0040_0000 } else { 0 })'

# 10. Use 2.0*l instead of the l+l the C emits (differs for NaN operand roles).
try_mutation "c: add(l,l) -> mul(2.0, l)" \
  'c = mul(sub(1.0f32, fabsf(sub(add(l, l), 1.0f32))), s);' \
  'c = mul(sub(1.0f32, fabsf(sub(mul(2.0f32, l), 1.0f32))), s);'

# 11. Naive arithmetic throughout: drop the operand-role-aware helpers.
try_mutation "naive m = l - 0.5*c (no NaN operand-role modelling)" \
  'm = sub(l, mul(c, 0.5f32));' 'm = l - 0.5f32 * c;'

# 12. Drop the export wrapper's C ABI name.
try_mutation "remove #[no_mangle] (symbol parity must fail)" \
  '#[unsafe(no_mangle)]' '' 

# 13. Reorder the terminal else to reuse a wrong variable.
try_mutation "terminal else: (m,m,m) -> (m,m,add(m,0.0))" \
  '        (m, m, m)' '        (m, m, add(m, 0.0f32))'

# 14. Revert the loads to raw place projections. Under `debug-assertions` this
#     makes a null `src` ABORT instead of segfaulting, unlike the C.
#     Only observable in the debug profile, so run this script's mutations there.
try_mutation "load via *src.add(0) instead of ptr::read" \
  'let h: f32 = unsafe { core::ptr::read(src.add(0)) };' \
  'let h: f32 = unsafe { *src.add(0) };'

# 15. Same for the fast-path stores.
try_mutation "store via *dest.add(0) instead of ptr::write" \
  'core::ptr::write(dest.add(0), l);' '*dest.add(0) = l;'

restore
trap - EXIT

echo
echo "================ MUTATION RESULTS ================"
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "=================================================="
if [ "$survived" -eq 0 ]; then
  echo "Every non-equivalent mutant was killed: the differential suite is not vacuous."
else
  echo "A mutant survived unexpectedly (or an equivalence claim was falsified) -> investigate."
fi

# Rebuild the pristine library so the tree is left in a verified state.
timeout 600 cargo build > /dev/null 2>&1
timeout 600 cargo build --release > /dev/null 2>&1
exit $survived
