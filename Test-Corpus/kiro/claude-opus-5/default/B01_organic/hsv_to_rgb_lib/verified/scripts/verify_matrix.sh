#!/usr/bin/env bash
# Full verification matrix: build the C reference, then run every phase of the
# differential suite under every cargo feature combination and both profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT/translation" || exit 1

fail=0
run() { # run <label> <cmd...>
  local label="$1"; shift
  echo "=== $label"
  if timeout 600 "$@" >/tmp/vm.log 2>&1; then
    grep -E '^test result:' /tmp/vm.log | sed 's/^/    /'
  else
    echo "    FAILED: $*"
    tail -n 40 /tmp/vm.log | sed 's/^/    /'
    fail=1
  fi
}

# --- C reference -------------------------------------------------------------
echo "=== building C reference"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
ls "$ROOT/c_src/build/"lib*.so

# --- feature combinations ---------------------------------------------------
# Every declared feature name, if any.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== Cargo.toml declares no [features]; default is the only configuration"
  COMBOS+=("--no-default-features" "" "--all-features")
else
  # Power set of the declared features, plus the default and all-features builds.
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((m = 0; m < total; m++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((m >> b) & 1)) && sel+=("${FEATURES[$b]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("" "--all-features")
fi

# --- matrix -----------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  read -r -a cargs <<<"$combo"
  label="${combo:-<default>}"

  # cargo check first, so a broken configuration is reported as such.
  run "check   [$label]" cargo check "${cargs[@]}"

  # Build BOTH cdylib profiles: the harness loads whichever exist and compares
  # each against C, so release and debug codegen are both covered.
  run "build   [$label] release" cargo build --release "${cargs[@]}"
  run "build   [$label] debug"   cargo build "${cargs[@]}"

  run "test    [$label] dev"     cargo test "${cargs[@]}"
  run "test    [$label] release" cargo test --release "${cargs[@]}"
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
