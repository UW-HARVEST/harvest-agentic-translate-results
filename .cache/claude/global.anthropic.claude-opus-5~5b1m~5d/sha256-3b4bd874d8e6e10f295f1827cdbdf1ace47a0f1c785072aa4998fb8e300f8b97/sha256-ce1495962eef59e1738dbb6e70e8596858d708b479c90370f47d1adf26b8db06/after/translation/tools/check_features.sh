#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination and
# under both build profiles. Feature names are extracted from Cargo.toml rather
# than hard-coded, so a newly added feature is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CARGO_FLAGS="--offline"
# Use a writable scratch dir (TMPDIR may be sandboxed away from /tmp).
LOGDIR="${TMPDIR:-target}"
mkdir -p "$LOGDIR" 2>/dev/null || LOGDIR="target"
LOG="$LOGDIR/feature-combo-$$.log"
fail=0

# ---- 1. Build the C reference .so ----------------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C reference library =="
  ( mkdir -p ../c_src/build && cd ../c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---- 2. Enumerate features from Cargo.toml ------------------------------
# Everything between the [features] header and the next [section] header.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "== features declared in Cargo.toml: ${#FEATURES[@]} =="
if [ "${#FEATURES[@]}" -gt 0 ]; then
  printf '   - %s\n' "${FEATURES[@]}"
fi

# ---- 3. Build the combination list --------------------------------------
# Always include the default build. If features exist, add: no-default-features
# alone, each feature alone, and the full powerset (capped so the run stays
# inside the time budget).
COMBOS=("DEFAULT")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("NONE")
  n=${#FEATURES[@]}
  if [ "$n" -le 10 ]; then
    total=$((1 << n))
    for ((mask = 1; mask < total; mask++)); do
      combo=""
      for ((i = 0; i < n; i++)); do
        if (((mask >> i) & 1)); then combo="$combo,${FEATURES[$i]}"; fi
      done
      COMBOS+=("${combo#,}")
    done
  else
    echo "   (>10 features: testing each feature individually plus all-features)"
    for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
    COMBOS+=("ALL")
  fi
fi

echo "== ${#COMBOS[@]} combination(s) x 2 profile(s) =="

run_one() { # $1 = combo label, $2 = profile
  local combo="$1" profile="$2" args=()
  case "$combo" in
    DEFAULT) ;;
    NONE) args+=(--no-default-features) ;;
    ALL)  args+=(--all-features) ;;
    *)    args+=(--no-default-features --features "$combo") ;;
  esac
  [ "$profile" = "release" ] && args+=(--release)

  echo
  echo "---- combo=[$combo] profile=[$profile] ----"
  # The cdylib must exist in the matching profile: cargo test does not build a
  # cdylib-only lib target, so build it explicitly first.
  if ! timeout 600 cargo build $CARGO_FLAGS "${args[@]}" >/dev/null 2>&1; then
    echo "BUILD FAILED: combo=$combo profile=$profile"
    timeout 600 cargo build $CARGO_FLAGS "${args[@]}" 2>&1 | tail -20
    fail=1
    return
  fi
  if ! timeout 600 cargo test $CARGO_FLAGS "${args[@]}" 2>&1 | tee "$LOG" \
       | grep -E '^(test result|error)'; then :; fi
  if grep -qE '^(test result: FAILED|error(\[|:))' "$LOG"; then
    echo "TESTS FAILED: combo=$combo profile=$profile"
    grep -E '^(test .* FAILED|---- )' "$LOG" | head -20
    fail=1
  fi
  rm -f "$LOG"
}

for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    run_one "$combo" "$profile"
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED (${#COMBOS[@]} combo(s) x 2 profiles)"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
