#!/usr/bin/env bash
# Full verification driver: symbol parity + Phase B/C differential tests across
# every feature combination and both profiles.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "############ 0. build the C shared library ############"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (powerset of [features] keys).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{
        sub(/[ ]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)
echo "############ 1. feature surface ############"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "no [features] declared -> single configuration (default)"
  COMBOS=("__default__")
else
  echo "features: ${FEATURES[*]}"
  COMBOS=("__default__" "__none__")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'combination: %s\n' "${COMBOS[@]}"

run() { # run <label> <profile-flag> <feature-flags...>
  local label="$1"; shift
  echo
  echo "===== $label ====="
  # The cdylib must exist for the profile under test before tests dlopen it.
  cargo build "$@" >/dev/null 2>&1 || { echo "BUILD FAILED: $label"; FAIL=1; return; }
  if timeout 600 cargo test "$@" 2>&1 | tail -25; then :; fi
  # capture the real status of cargo test (tail masks it)
  timeout 600 cargo test "$@" >/dev/null 2>&1
  local st=$?
  if [ $st -ne 0 ]; then echo ">>> FAILED ($st): $label"; FAIL=1; else echo ">>> PASSED: $label"; fi
}

echo
echo "############ 2. tests: every combination x {debug, release} ############"
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) fflags=() ; desc="default-features" ;;
    __none__)    fflags=(--no-default-features) ; desc="no-default-features" ;;
    *)           fflags=(--no-default-features --features "$combo") ; desc="features=$combo" ;;
  esac
  run "debug   | $desc" "${fflags[@]}"
  run "release | $desc" --release "${fflags[@]}"
done

echo
echo "############ 3. symbol parity (nm -D) ############"
for prof in debug release; do
  RS_SO="target/$prof/libflip_horizontal_lib.so"
  [ -f "$RS_SO" ] || { echo "$prof: cdylib missing, skipped"; continue; }
  missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u))"
  if [ -z "$missing" ]; then
    echo "$prof: OK - 0 symbols missing from the Rust .so"
  else
    echo "$prof: MISSING SYMBOLS:"; echo "$missing"; FAIL=1
  fi
done

echo
if [ $FAIL -eq 0 ]; then echo "############ ALL CHECKS PASSED ############";
else echo "############ THERE WERE FAILURES ############"; fi
exit $FAIL
