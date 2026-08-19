#!/usr/bin/env bash
# Full verification matrix.
#
#   Phase A : artifacts + symbol parity            (build_all.sh, SYMBOLS.md)
#   Phase B : valid-path differential tests        (tests/configs.rs, CONFIGS.md)
#   Phase C : error-path differential tests        (tests/errors.rs,  ERRORS.md)
#   Phase D : symbol parity + every feature combo  (tests/symbols.rs)
#
# Every feature combination is derived from Cargo.toml, not hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

LOGDIR="target/verify-logs"
mkdir -p "$LOGDIR"
fails=0
nstep=0
step() {
  local desc="$1"; shift
  nstep=$((nstep+1))
  local log="$LOGDIR/step-$(printf '%02d' $nstep).log"
  printf '\n>>> [%02d] %s\n' "$nstep" "$desc"
  if timeout 600 "$@" >"$log" 2>&1; then
    grep -E '^(test result|    missing|OK|caught )' "$log" | tail -n 8
    echo "    PASS  (log: $log)"
  else
    tail -n 30 "$log"
    echo "    FAIL: $desc  (log: $log)"
    fails=$((fails+1))
  fi
}

# --- enumerate the feature combinations from Cargo.toml -----------------------
mapfile -t FEATURES < <(cargo metadata --offline --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys
for f in json.load(sys.stdin)["packages"][0]["features"]:
    print(f)' | grep -v '^[[:space:]]*$')

echo "=== feature axes declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none}) ==="

# With N declared features the combinations are the powerset; with none it is the
# single empty set. --all-features and the default set are checked as well
# because they are distinct cargo invocations even when they resolve to the same
# feature set.
COMBOS=("--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    list=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && list="${list:+$list,}${FEATURES[$i]}"
    done
    COMBOS+=("--no-default-features --features $list")
  done
fi
COMBOS+=("")               # default feature set
COMBOS+=("--all-features")

echo "=== ${#COMBOS[@]} cargo configurations to verify ==="
for c in "${COMBOS[@]}"; do echo "      cargo <cmd> ${c:-<default>}"; done

# --- Phase 2 of the brief: cargo check for EVERY combination ------------------
for prof in "" "--release"; do
  for c in "${COMBOS[@]}"; do
    # shellcheck disable=SC2086
    step "cargo check $prof ${c:-<default>}" cargo check --offline $prof $c --all-targets
  done
done

# --- Phases A-D per profile ---------------------------------------------------
for prof in "" "--release"; do
  # shellcheck disable=SC2086
  step "build all artifacts + symbol parity $prof" ./build_all.sh $prof
  for c in "${COMBOS[@]}"; do
    # shellcheck disable=SC2086
    step "cargo test $prof ${c:-<default>}" cargo test --offline $prof $c
  done
done

# --- test-suite sensitivity ---------------------------------------------------
step "mutation sensitivity of the suite" ./mutation_check.sh

# Leave the tree in the default (debug) state.
./build_all.sh >/dev/null 2>&1

echo
if [ "$fails" -eq 0 ]; then
  echo "================ ALL CONFIGURATIONS VERIFIED ================"
else
  echo "================ $fails STEP(S) FAILED ================"
  exit 1
fi
