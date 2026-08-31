#!/usr/bin/env bash
# Build the C reference library and the Rust cdylib, then run the differential
# tests for every valid Cargo feature combination.
#
# `Cargo.toml` declares no [features] section and `c_src/CMakeLists.txt` exposes
# no build options, so the combination set is just the default configuration.
# The loop below is derived from Cargo.toml rather than hardcoded, so it keeps
# working if features are added later.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

fail=0

echo "=== Building C reference shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "ok: $(ls c_src/build/libdriver.so)"

# --- Enumerate feature combinations -----------------------------------------
# Collect declared feature names (excluding the implicit "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' translation/Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== No [features] declared: single configuration (default) ==="
  COMBOS+=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
  echo "=== ${#COMBOS[@]} feature combination(s) from ${FEATURES[*]} ==="
fi

cd translation

run_combo() {
  local combo="$1" label="$2"
  shift 2
  local -a fargs=("$@")

  echo
  echo "############ configuration: $label ############"

  echo "--- cargo check ---"
  if ! timeout 600 cargo check "${fargs[@]}" 2>&1 | tail -5; then
    echo "CHECK FAILED: $label"; fail=1; return
  fi

  # cargo test does not rebuild cdylib artifacts, so build the .so explicitly.
  echo "--- cargo build (cdylib) ---"
  if ! timeout 600 cargo build "${fargs[@]}" 2>&1 | tail -5; then
    echo "BUILD FAILED: $label"; fail=1; return
  fi

  echo "--- symbol parity (nm -D) ---"
  nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u > /tmp/csyms.$$
  nm -D --defined-only target/debug/libdriver.so   | awk '{print $3}' | sort -u > /tmp/rsyms.$$
  missing="$(comm -23 /tmp/csyms.$$ /tmp/rsyms.$$)"
  if [ -n "$missing" ]; then
    echo "MISSING EXPORTS in Rust .so for $label:"; echo "$missing"; fail=1
  else
    echo "ok: Rust .so exports every symbol the C .so exports"
  fi
  rm -f /tmp/csyms.$$ /tmp/rsyms.$$

  echo "--- cargo test ---"
  if ! timeout 600 cargo test "${fargs[@]}" 2>&1 | tail -25; then
    echo "TEST FAILED: $label"; fail=1; return
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run_combo "$combo" "--no-default-features" --no-default-features
  else
    run_combo "$combo" "--no-default-features --features $combo" \
      --no-default-features --features "$combo"
  fi
done

# Also exercise the default feature set and the everything-on set.
run_combo "" "default features" 
run_combo "" "--all-features" --all-features

echo
if [ "$fail" -ne 0 ]; then
  echo "########## RESULT: FAILURES PRESENT ##########"
  exit 1
fi
echo "########## RESULT: all configurations pass ##########"
