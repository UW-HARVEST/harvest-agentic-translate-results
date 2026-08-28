#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# build-time feature combination.
#
#   ./verify_features.sh
#
# Enumerates [features] from Cargo.toml, then for each subset runs
# `cargo check` and `cargo test` with --no-default-features --features <combo>.
# The crate currently declares no features, so the single valid configuration
# is the empty set; the loop keeps working if features are added later.
set -uo pipefail

cd "$(dirname "$0")"
root="$(cd .. && pwd)"

# --- build the C shared library (ground truth) ------------------------------
if ! ls "$root"/c_src/build/*.so >/dev/null 2>&1; then
  echo "== building C shared library =="
  (mkdir -p "$root/c_src/build" && cd "$root/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1) \
    || { echo "C build FAILED (see /tmp/cmake.log)"; exit 1; }
fi

# --- enumerate feature combinations ----------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)

combos=("")                      # the empty (no-features) configuration
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    combos+=("$combo")
  done
fi

echo "== ${#combos[@]} feature combination(s) to verify: ${combos[*]:-<none>} =="

status=0
for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  for profile in "" --release; do
    tag="$label ${profile:---debug}"
    echo "-- cargo check   [$tag]"
    if ! timeout 600 cargo check --all-targets "${args[@]}" $profile >/tmp/check.log 2>&1; then
      echo "   CHECK FAILED [$tag]"; tail -30 /tmp/check.log; status=1; continue
    fi
    echo "-- cargo test    [$tag]"
    if ! timeout 600 cargo test "${args[@]}" $profile >/tmp/test.log 2>&1; then
      echo "   TEST FAILED [$tag]"; grep -E "panicked|assertion|FAILED|differs" /tmp/test.log | head -20
      status=1; continue
    fi
    grep -E "^test result" /tmp/test.log | sed 's/^/   /'
  done
done

if (( status == 0 )); then
  echo "== ALL COMBINATIONS PASS =="
else
  echo "== FAILURES PRESENT =="
fi
exit $status
