#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every feature
# combination and every cdylib profile.
#
# The crate declares no [features] table (verified below), so the complete set
# of feature combinations is {default, --no-default-features}. The cdylib
# profiles (dev / release / ubcheck) are swept inside the tests themselves via
# TRANSLATION_PROFILES.
set -uo pipefail

cd "$(dirname "$0")"

FAIL=0
run() {
  local desc="$1"; shift
  echo "=============================================================="
  echo "== $desc"
  echo "=============================================================="
  if timeout 600 "$@"; then
    echo "-- PASS: $desc"
  else
    echo "-- FAIL: $desc"
    FAIL=1
  fi
  echo
}

# ---------------------------------------------------------------- feature set
# Mechanically extract the feature combinations from Cargo.toml.
FEATURES=$(python3 - <<'PY'
import re
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
print(" ".join(feats))
PY
)
if [ -n "$FEATURES" ]; then
  echo "declared features: $FEATURES"
else
  echo "declared features: (none) -> combinations are {default, --no-default-features}"
fi

# ------------------------------------------------------------ build both .so's
echo "## building the C shared library"
( mkdir -p ../c_src/build \
  && cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
ls -l ../c_src/build/*.so

# ------------------------------------------------------------------- the sweep
COMBOS=("" "--no-default-features")
if [ -n "$FEATURES" ]; then
  # every single feature on its own, plus all features together
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  # forwarded to the nested `cargo build` that produces the cdylibs
  export TRANSLATION_FEATURE_ARGS="$combo"
  # nested builds go to fresh dirs so the combo really is rebuilt
  rm -rf target/so-dev target/so-rel target/so-ubc

  unset TRANSLATION_PROFILES
  run "cargo check [$label]" cargo check --offline --tests $combo
  run "cargo test  [$label] profiles=all" cargo test --offline $combo -- --test-threads=4

  # and once more with the profiles selected explicitly, one at a time, so a
  # profile-specific divergence cannot hide behind another profile
  for prof in dev release ubcheck; do
    export TRANSLATION_PROFILES="$prof"
    run "cargo test [$label] profile=$prof" cargo test --offline $combo -- --test-threads=4
  done
  unset TRANSLATION_PROFILES
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
