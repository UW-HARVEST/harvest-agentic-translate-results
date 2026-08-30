#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration.
#
#   ./verify.sh
#
# Steps:
#   1. enumerate every valid feature combination declared in Cargo.toml
#   2. `cargo check` each combination
#   3. build the C shared library
#   4. `cargo test` each combination, in both dev and release profiles
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
timeout_s=600
rc=0

# --- 1. enumerate feature combinations -------------------------------------
# Read the [features] table, dropping "default" (covered by the default build)
# and any `dep:`/optional-dependency implicit entries.
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); print $0
    }
  ' "$here/Cargo.toml" | grep -vx default
)

combos=()
n=${#features[@]}
if [ "$n" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default build."
  combos=("")
else
  # Every subset of the feature set (2^n), including the empty one.
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${features[i]}"
      fi
    done
    combos+=("$combo")
  done
fi

echo "Feature combinations to verify: ${#combos[@]}"
for c in "${combos[@]}"; do echo "  - ${c:-<none>}"; done

# --- 2. cargo check every combination --------------------------------------
echo
echo "=== cargo check ==="
for c in "${combos[@]}"; do
  args=(--no-default-features)
  [ -n "$c" ] && args+=(--features "$c")
  if timeout "$timeout_s" cargo check --manifest-path "$here/Cargo.toml" "${args[@]}" \
      >/tmp/check.log 2>&1; then
    echo "  OK    features=${c:-<none>}"
  else
    echo "  FAIL  features=${c:-<none>}"
    tail -n 30 /tmp/check.log
    rc=1
  fi
done

# --- 3. build the C shared library -----------------------------------------
echo
echo "=== build C shared library ==="
(
  mkdir -p "$root/c_src/build" &&
    cd "$root/c_src/build" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 &&
    cmake --build . >>/tmp/cmake.log 2>&1
) || {
  echo "  FAIL: see /tmp/cmake.log"
  tail -n 30 /tmp/cmake.log
  exit 1
}
echo "  OK    $root/c_src/build/libdriver.so"

# --- 4. cargo test every combination, both profiles ------------------------
echo
echo "=== cargo test ==="
for profile in dev release; do
  for c in "${combos[@]}"; do
    args=(--no-default-features)
    [ "$profile" = release ] && args+=(--release)
    [ -n "$c" ] && args+=(--features "$c")
    if timeout "$timeout_s" cargo test --manifest-path "$here/Cargo.toml" "${args[@]}" \
        >/tmp/test.log 2>&1; then
      echo "  OK    profile=$profile features=${c:-<none>}  ($(grep -cE '^test .* ok$' /tmp/test.log) tests)"
    else
      echo "  FAIL  profile=$profile features=${c:-<none>}"
      sed -n '/^failures:/,$p' /tmp/test.log | head -n 60
      rc=1
    fi
  done
done

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL CONFIGURATIONS MATCH THE C IMPLEMENTATION"
else
  echo "FAILURES PRESENT"
fi
exit "$rc"
