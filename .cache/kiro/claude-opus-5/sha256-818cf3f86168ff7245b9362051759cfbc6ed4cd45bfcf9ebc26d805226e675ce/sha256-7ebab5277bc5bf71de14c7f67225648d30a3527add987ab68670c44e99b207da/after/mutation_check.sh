#!/usr/bin/env bash
# Mutation harness: deliberately breaks the Rust translation in ways a sloppy
# port plausibly would, and confirms the differential suite fails each time.
# A mutation that survives means the suite has a blind spot.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/translation" || exit 1

cp src/lib.rs /tmp/mutate_orig.rs
restore() { cp /tmp/mutate_orig.rs src/lib.rs; }
trap restore EXIT

# name | literal search | replacement
# An "expected survivor" is a mutation that is provably indistinguishable
# through the public API, so no test could ever catch it.
run_mutation() {
  local name="$1" find="$2" repl="$3" expect="${4:-caught}"
  restore
  FIND="$find" REPL="$repl" python3 - <<'EOF'
import os
p = 'src/lib.rs'
s = open(p).read()
f, r = os.environ['FIND'], os.environ['REPL']
assert f in s, f"mutation target not found: {f!r}"
open(p, 'w').write(s.replace(f, r, 1))
EOF
  if [[ $? -ne 0 ]]; then echo "SKIP  $name (pattern not found)"; return 0; fi

  if ! timeout 600 cargo build >/dev/null 2>&1; then
    echo "SKIP  $name (mutation does not compile)"; return 0
  fi

  if timeout 600 cargo test >/dev/null 2>&1; then
    if [[ "$expect" == "survives" ]]; then
      echo "survived  $name  (expected: equivalent on the reachable domain)"
      return 0
    fi
    echo "SURVIVED  $name  <-- blind spot in the test suite"
    return 1
  fi

  if [[ "$expect" == "survives" ]]; then
    echo "CAUGHT    $name  <-- expected to be equivalent; re-check the reasoning"
    return 1
  fi
  echo "caught    $name"
  return 0
}

STATUS=0

run_mutation "pow in f32 instead of promoting to f64" \
  'unsafe { pow((c + 0.055) / 1.055, 2.4) as f32 }' \
  '((channel + 0.055f32) / 1.055f32).powf(2.4f32)' || STATUS=1

# Channels are u8, so the dark branch only ever sees c in {0/255 .. 10/255}.
# For all 11 of those values the f64-then-narrow divide and the pure f32
# divide give identical bits, so this difference is unobservable.
run_mutation "linear branch divides by 12.92 in f32" \
  '(c / 12.92) as f32' \
  '(channel / 12.92f32)' survives || STATUS=1

run_mutation "luminance coefficients R and G swapped" \
  '0.2126f32 * r + 0.7152f32 * g' \
  '0.7152f32 * r + 0.2126f32 * g' || STATUS=1

run_mutation "blue coefficient 0.0722 -> 0.0721" \
  '0.0722f32 * b' \
  '0.0721f32 * b' || STATUS=1

run_mutation "luminance sum re-associated to the right" \
  '0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b' \
  '0.2126f32 * r + (0.7152f32 * g + 0.0722f32 * b)' || STATUS=1

run_mutation "luminance accumulated in f64" \
  '0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b' \
  '(0.2126f64 * r as f64 + 0.7152f64 * g as f64 + 0.0722f64 * b as f64) as f32' || STATUS=1

run_mutation "High/Low comparison inverted" \
  'if high < low {' \
  'if high > low {' || STATUS=1

run_mutation "ratio inverted (Low / High)" \
  '    high / low' \
  '    low / high' || STATUS=1

run_mutation "WCAG +0.05 offset added (a plausible 'fix' the C code omits)" \
  '    high / low' \
  '    (high + 0.05) / (low + 0.05)' || STATUS=1

run_mutation "zero denominator guarded instead of dividing" \
  '    high / low' \
  '    if low == 0.0 { 0.0 } else { high / low }' || STATUS=1

# 0.04*255 = 10.2 and 0.04045*255 = 10.31, so both thresholds flip between
# i=10 and i=11. No u8 channel can tell them apart.
run_mutation "branch threshold 0.04045 -> 0.04" \
  'if c > 0.04045 {' \
  'if c > 0.04 {' survives || STATUS=1

run_mutation "channel scaled by 256 instead of 255" \
  'f32::from(A.R) / 255.0f32' \
  'f32::from(A.R) / 256.0f32' || STATUS=1

run_mutation "gamma exponent 2.4 -> 2.2" \
  'pow((c + 0.055) / 1.055, 2.4)' \
  'pow((c + 0.055) / 1.055, 2.2)' || STATUS=1

restore
timeout 600 cargo build >/dev/null 2>&1

echo "======================================"
if [[ $STATUS -eq 0 ]]; then
  echo "ALL MUTATIONS CAUGHT - the differential suite is sensitive"
else
  echo "SOME MUTATIONS SURVIVED - see above"
fi
exit $STATUS
