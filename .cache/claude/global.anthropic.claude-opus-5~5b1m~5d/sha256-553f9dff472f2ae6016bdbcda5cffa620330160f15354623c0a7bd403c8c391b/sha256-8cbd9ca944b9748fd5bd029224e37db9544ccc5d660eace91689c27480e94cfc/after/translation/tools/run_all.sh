#!/usr/bin/env bash
# Full verification run: build both libraries, then run the whole differential
# suite under EVERY cargo feature combination declared by Cargo.toml.
set -euo pipefail

CRATE="$(cd "$(dirname "$0")/.." && pwd)"
ROOT="$(dirname "$CRATE")"

echo "== building the C shared library =="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$(ls "$ROOT"/c_src/build/lib*.so)"
echo "   $C_SO"

echo "== enumerating feature combinations =="
# Every feature named in the [features] table of Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' "$CRATE/Cargo.toml" | grep -v '^default$' || true)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "   no [features] table -> the only combination is the default (empty) one"
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  COMBOS+=("all:--all-features")
else
  # Full power set of the declared features, with and without defaults.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((k = 0; k < n; k++)); do
      (((mask >> k) & 1)) && sel+=("${FARR[k]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("default+[$joined]:--features=$joined")
    COMBOS+=("only[$joined]:--no-default-features --features=$joined")
  done
fi

status=0
for combo in "${COMBOS[@]}"; do
  label="${combo%%:*}"; flags="${combo#*:}"
  echo
  echo "== feature combination: $label  (cargo $flags) =="

  # The Rust cdylib under test must be rebuilt for each combination.
  # shellcheck disable=SC2086
  (cd "$CRATE" && cargo build --release --offline $flags >/dev/null)
  # shellcheck disable=SC2086
  if (cd "$CRATE" && cargo test --offline $flags -- --test-threads=4); then
    echo "-- $label: PASS"
  else
    echo "-- $label: FAIL"
    status=1
  fi
done

echo
echo "== binary/driver check =="
if grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml" || [ -f "$CRATE/src/main.rs" ] \
   || grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "   a binary target exists -- its stdout must be diffed as well"
  status=1
else
  echo "   neither project builds an executable (CMakeLists.txt has only"
  echo "   add_library(... SHARED ...), the crate is crate-type=[\"cdylib\"]"
  echo "   with no [[bin]] and no src/main.rs) -> no stdout to compare"
fi

echo
echo "== negative control (mutation testing of the harness) =="
bash "$CRATE/tools/negative_control.sh" || status=1

echo
if [ "$status" -eq 0 ]; then echo "ALL PHASES PASS"; else echo "FAILURES PRESENT"; fi
exit "$status"
