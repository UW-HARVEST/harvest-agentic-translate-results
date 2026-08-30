#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# feature combination declared in translation/Cargo.toml.
#
# Usage: ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_SO="$ROOT/c_src/build/libSimpleList.so"
TIMEOUT=600
fail=0

step() { printf '\n=== %s ===\n' "$*"; }
run()  { timeout "$TIMEOUT" "$@"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
step "Feature enumeration"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' "$ROOT/translation/Cargo.toml"
)
# "default" is not an independently selectable feature.
SELECTABLE=()
for f in "${FEATURES[@]:-}"; do
  [[ -n "$f" && "$f" != "default" ]] && SELECTABLE+=("$f")
done

if ((${#SELECTABLE[@]} == 0)); then
  echo "No selectable features declared -> exactly one configuration."
  COMBOS=("")
else
  echo "Selectable features: ${SELECTABLE[*]}"
  COMBOS=()
  n=${#SELECTABLE[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo+=("${SELECTABLE[i]}")
    done
    COMBOS+=("$(IFS=,; echo "${combo[*]}")")
  done
fi
printf 'Combination count: %d\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
step "Build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && run cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && run cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "built: $C_SO"

# ---------------------------------------------------------------------------
# 3-5. Per combination: check, build, symbol diff, differential tests.
# ---------------------------------------------------------------------------
c_syms="$(mktemp)"
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtWwDdBbRrVv]$/ {print $3}' | sort -u > "$c_syms"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  feat_args=(--no-default-features)
  [[ -n "$combo" ]] && feat_args+=(--features "$combo")

  step "Combination: $label"

  ( cd "$ROOT/translation" && run cargo check "${feat_args[@]}" ) \
    || { echo "cargo check FAILED for $label"; fail=1; continue; }

  for profile in debug release; do
    build_args=("${feat_args[@]}")
    [[ "$profile" == release ]] && build_args+=(--release)

    ( cd "$ROOT/translation" && run cargo build "${build_args[@]}" ) >/dev/null \
      || { echo "cargo build ($profile) FAILED for $label"; fail=1; continue; }

    rust_so="$ROOT/translation/target/$profile/libSimpleList.so"

    # Symbol parity: every symbol the C .so defines must be defined here too.
    missing="$(comm -23 "$c_syms" \
      <(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TtWwDdBbRrVv]$/ {print $3}' | sort -u))"
    if [[ -n "$missing" ]]; then
      echo "MISSING EXPORTS ($profile, $label):"; echo "$missing"; fail=1
    else
      echo "symbol parity OK ($profile)"
    fi

    # Differential tests against the C .so, loading this exact artifact.
    ( cd "$ROOT/translation" \
      && SIMPLELIST_RUST_SO="$rust_so" run cargo test "${feat_args[@]}" -- --test-threads=4 ) \
      >/tmp/verify_test_$profile.log 2>&1
    if (($? == 0)); then
      echo "tests OK ($profile vs C): $(grep -c '^test .* ok$' /tmp/verify_test_$profile.log) cases"
    else
      echo "TESTS FAILED ($profile, $label):"; tail -40 /tmp/verify_test_$profile.log; fail=1
    fi
  done
done

step "Result"
if ((fail == 0)); then
  echo "PASS: all ${#COMBOS[@]} configuration(s) match the C implementation."
else
  echo "FAIL: see errors above."
fi
exit "$fail"
