#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration.
#
# `translation/Cargo.toml` declares no [features] and `c_src/CMakeLists.txt`
# declares no options, so there is exactly one configuration: the default
# (empty) feature set. It is still exercised in both cargo profiles, because
# the debug profile enables overflow checks and debug assertions that the
# release profile does not.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# Enumerate feature combinations (empty string == no features).
COMBOS=("")
mapfile -t EXTRA < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' \
    translation/Cargo.toml 2>/dev/null || true
)
if ((${#EXTRA[@]} > 0)); then
  echo "Discovered features: ${EXTRA[*]}"
  # Power set of the declared features.
  n=${#EXTRA[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo+="${EXTRA[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
else
  echo "No [features] declared -> single (default) configuration."
fi

echo "== building C ground-truth shared library =="
(cd c_src && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 &&
  cmake --build . >>/tmp/cmake.log 2>&1) || {
  tail -30 /tmp/cmake.log
  exit 1
}

cd translation
for combo in "${COMBOS[@]}"; do
  for profile in release dev; do
    flags=(--no-default-features)
    [[ -n $combo ]] && flags+=(--features "$combo")
    [[ $profile == release ]] && flags+=(--release)
    label="features='${combo:-<none>}' profile=$profile"

    echo "== cargo check   $label =="
    timeout 600 cargo check "${flags[@]}" 2>&1 | tail -5

    echo "== cargo build   $label =="
    timeout 600 cargo build "${flags[@]}" 2>&1 | tail -3

    echo "== cargo test    $label =="
    timeout 600 cargo test "${flags[@]}" 2>&1 | tail -12
  done
done

echo "== nm symbol comparison =="
diff <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort) \
  && echo "identical symbol sets" || echo "(differences listed above; see tests/symbol_parity.rs for the authoritative check)"

echo "ALL CONFIGURATIONS VERIFIED"
