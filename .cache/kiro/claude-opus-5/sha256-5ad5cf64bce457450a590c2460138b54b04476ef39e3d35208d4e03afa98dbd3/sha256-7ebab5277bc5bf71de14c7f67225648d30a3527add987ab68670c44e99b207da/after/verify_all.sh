#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination for the `driver` crate and
# runs `cargo check` plus `cargo test` for each, in both dev and release
# profiles. Run from the repository root or anywhere; paths are resolved here.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
TIMEOUT="${TIMEOUT:-600}"

# --- Ensure the C reference library exists -----------------------------------
if [[ ! -f "$ROOT/c_src/build/libdriver.so" ]]; then
  echo "== building C reference library =="
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# --- Enumerate features declared in Cargo.toml ------------------------------
# Reads the [features] table; ignores the implicit "default" pseudo-feature.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=");
      gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
    }
  ' "$CRATE/Cargo.toml"
)

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Build the list of combinations to test: the powerset of declared features,
# each run with --no-default-features, plus the plain default configuration.
COMBOS=("<default>")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[$i]},"
      fi
    done
    COMBOS+=("${combo%,}")
  done
else
  # No features declared: the only other configuration is the empty one.
  COMBOS+=("")
fi

# --- Run check and test for each combination --------------------------------
FAILED=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "<default>" ]]; then
    FLAGS=()
    label="default features"
  elif [[ -z "$combo" ]]; then
    FLAGS=(--no-default-features)
    label="no features"
  else
    FLAGS=(--no-default-features --features "$combo")
    label="features: $combo"
  fi

  for profile in dev release; do
    PROFILE_FLAGS=()
    [[ "$profile" == "release" ]] && PROFILE_FLAGS=(--release)

    for cmd in check test; do
      printf '== cargo %-5s [%s] (%s) == ' "$cmd" "$label" "$profile"
      if ( cd "$CRATE" && timeout "$TIMEOUT" cargo "$cmd" \
             "${FLAGS[@]}" "${PROFILE_FLAGS[@]}" >/tmp/sweep.log 2>&1 ); then
        echo "OK"
      else
        echo "FAILED"
        tail -n 30 /tmp/sweep.log
        FAILED=1
      fi
    done
  done
done

echo
if (( FAILED )); then
  echo "RESULT: at least one configuration failed"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) pass check + test in dev and release"
