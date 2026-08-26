#!/bin/bash
# Run the full verification matrix: every Cargo feature combination gets
# `cargo check`ed, then (for the combinations that map onto a buildable C
# configuration) built and differentially tested against the C shared object.
#
#   ./check_all.sh check    -- cargo check only
#   ./check_all.sh test     -- build + cargo test, debug profile (default)
#   ./check_all.sh release  -- build + cargo test, release profile
#
# Every Rust artifact is rebuilt for the exact feature set before its tests run,
# so a test can never pass against a stale libdriver.so.
#
# The debug profile is the stricter of the two: there, a plain `a + b` on
# overflowing operands panics, so the INT_MIN/INT_MAX rows prove the translation
# really uses wrapping arithmetic. The release profile additionally exercises
# `panic = "abort"` and optimised codegen.

set -u
cd "$(dirname "$0")" || exit 1

MODE="${1:-test}"
PROFILE_FLAG=""
if [ "$MODE" = "release" ]; then
  PROFILE_FLAG="--release"
fi
LOG_DIR="${TMPDIR:-/tmp}/verify_logs"
mkdir -p "$LOG_DIR"

combos() {
  # The valid matrix: one OP feature (or none -> #ifndef default add) x one
  # REPEAT feature (or none -> #ifndef default 5).
  for op in "" add sub mul; do
    for rep in "" 0 1 2 3 4 5 6 7; do
      c=""
      [ -n "$op" ] && c="$op"
      if [ -n "$rep" ]; then
        if [ -n "$c" ]; then c="$c,$rep"; else c="$rep"; fi
      fi
      echo "$c"
    done
  done
  # Cargo features are additive, so a consumer can also enable several members of
  # one family at once -- something CMake's single-valued cache variables cannot
  # express. These must still compile and resolve to exactly one deterministic
  # configuration (OP: mul > sub > add; REPEAT: lowest number wins). The test
  # harness applies the same precedence when it builds the C comparand, so these
  # are differentially tested too, not merely compiled.
  echo "add,sub"
  echo "add,mul"
  echo "sub,mul"
  echo "add,sub,mul"
  echo "3,5"
  echo "0,7"
  echo "mul,3,5"
  echo "add,sub,mul,0,1,2,3,4,5,6,7"
  echo "sub,7,2"
}

pass=0; fail=0; failed_combos=()

while read -r combo; do
  label="${combo:-<defaults>}"
  safe=$(echo "${combo:-defaults}" | tr ',' '_')
  log="$LOG_DIR/$safe.log"

  if ! timeout 300 cargo check --no-default-features --features "$combo" \
        $PROFILE_FLAG --all-targets > "$log" 2>&1; then
    echo "CHECK FAIL [$label]  (see $log)"
    fail=$((fail+1)); failed_combos+=("$label"); continue
  fi

  if [ "$MODE" = "check" ]; then
    echo "check ok    [$label]"
    pass=$((pass+1)); continue
  fi

  # Rebuild both artifacts for THIS feature set, then test against them.
  if ! timeout 300 cargo build --no-default-features --features "$combo" \
        $PROFILE_FLAG --lib --bin driver >> "$log" 2>&1; then
    echo "BUILD FAIL [$label]  (see $log)"
    fail=$((fail+1)); failed_combos+=("$label"); continue
  fi

  if ! timeout 600 cargo test --no-default-features --features "$combo" \
        $PROFILE_FLAG >> "$log" 2>&1; then
    echo "TEST FAIL  [$label]  (see $log)"
    grep -E "^(test .* FAILED|assertion|thread .* panicked)" "$log" | head -8
    fail=$((fail+1)); failed_combos+=("$label"); continue
  fi

  n=$(grep -hoE "^test result: ok\. [0-9]+" "$log" | awk '{s+=$4} END {print s}')
  echo "test ok     [$label]  ($n tests)"
  pass=$((pass+1))
done < <(combos)

echo "-----------------------------------------------"
echo "combinations passed: $pass   failed: $fail"
if [ "$fail" -ne 0 ]; then
  printf 'failed: %s\n' "${failed_combos[@]}"
  exit 1
fi
echo "ALL FEATURE COMBINATIONS OK ($MODE)"
