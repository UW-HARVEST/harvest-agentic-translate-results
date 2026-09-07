#!/usr/bin/env bash
# Full verification sweep: builds the C reference library, then runs the
# differential test suite under every feature combination and both profiles.
#
# Usage: ./scripts/verify_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="--offline"   # crates.io is unreachable in the sandbox; drop if online
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }

# --- 1. Build the C reference shared library -------------------------------
step "Building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { echo "missing $C_SO"; exit 1; }

# --- 2. Enumerate feature combinations -------------------------------------
# Read the [features] table out of Cargo.toml; if there is none, the only
# configuration is the default one.
step "Enumerating feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$HERE/Cargo.toml"
)
echo "declared features: ${FEATURES[*]:-<none>}"

# Build the list of --features arguments: the empty set, every individual
# feature, and the full set (the powerset for small n).
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="$combo,${FEATURES[$i]}"; fi
    done
    COMBOS+=("${combo#,}")
  done
fi
echo "combinations to test: ${#COMBOS[@]}"

# --- 3. Run the suite for each combination x profile ------------------------
run() { # run <label> <extra cargo args...>
  local label="$1"; shift
  step "TEST $label"
  # `cargo test` does NOT build the cdylib target, so without this the tests
  # would fall back to whatever libdriver.so another profile left behind and
  # silently verify the wrong artifact. Build it explicitly first.
  if ! timeout 600 cargo build $CARGO_FLAGS "$@" >/dev/null 2>&1; then
    echo "FAIL (build): $label"; FAILED=1; return
  fi
  # Note: do NOT pipe cargo directly into grep — the pipeline's exit status
  # would be grep's, which matches on almost any output and would turn a failed
  # test run into a reported PASS. Capture to a file and test cargo's own status.
  local log
  log="$(mktemp)"
  timeout 600 cargo test $CARGO_FLAGS "$@" >"$log" 2>&1
  local rc=$?
  grep -E "Running|test result|FAILED|^error" "$log"
  if [ "$rc" -eq 0 ]; then
    echo "PASS: $label"
  else
    echo "FAIL: $label (cargo exit $rc)"; tail -n 30 "$log"; FAILED=1
  fi
  rm -f "$log"
}

cd "$HERE" || exit 1
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
      run "default-features ${profile:-debug}" $profile
      run "no-default-features ${profile:-debug}" $profile --no-default-features
    else
      run "features=$combo ${profile:-debug}" $profile --no-default-features --features "$combo"
    fi
  done
  run "all-features ${profile:-debug}" $profile --all-features
done

# --- 4. Symbol parity ------------------------------------------------------
step "Symbol parity (nm -D)"
for prof in release debug; do
  RS_SO="$HERE/target/$prof/libdriver.so"
  [ -f "$RS_SO" ] || continue
  echo "--- $prof ---"
  diff <(nm -D --defined-only "$C_SO"  | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only "$RS_SO" | awk '{print $2, $3}' | sort) \
    && echo "symbol sets identical ($prof)" \
    || { echo "SYMBOL MISMATCH ($prof)"; FAILED=1; }
done

step "RESULT"
if [ "$FAILED" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$FAILED"
