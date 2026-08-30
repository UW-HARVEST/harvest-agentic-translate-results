#!/usr/bin/env bash
# Build the C reference library, enumerate every Cargo feature combination
# declared in translation/Cargo.toml, and run `cargo check` plus the full
# differential test suite for each one.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"

echo "== building C reference library =="
(
  cd "$CSRC" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
ls -1 "$CSRC"/build/*.so

# Feature names are read from the [features] table, ignoring the implicit
# "default" key. Absent or empty table => the default build is the only config.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

echo "== declared features: ${FEATURES[*]:-<none>} =="

# Every subset of the feature set, plus the plain default build.
COMBOS=("__default__")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((bit = 0; bit < n; bit++)); do
      if (( mask & (1 << bit) )); then
        combo="${combo:+$combo,}${FEATURES[$bit]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

status=0
cd "$CRATE" || exit 1

for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    label="default features"
    args=()
  elif [[ -z "$combo" ]]; then
    label="no features"
    args=(--no-default-features)
  else
    label="features: $combo"
    args=(--no-default-features --features "$combo")
  fi

  for profile in dev release; do
    profile_args=()
    [[ "$profile" == "release" ]] && profile_args=(--release)

    echo
    echo "=============================================================="
    echo "== $label | profile: $profile"
    echo "=============================================================="

    if ! timeout 600 cargo check "${args[@]}" "${profile_args[@]}" 2>&1 | tail -n 15; then
      echo "RESULT: cargo check FAILED [$label | $profile]"
      status=1
      continue
    fi

    # The crate is cdylib-only, so nothing in the test targets links it and
    # `cargo test` will not build it. Build it explicitly so the tests have a
    # Rust .so to dlopen for this exact configuration.
    if ! timeout 600 cargo build "${args[@]}" "${profile_args[@]}" 2>&1 | tail -n 15; then
      echo "RESULT: cargo build FAILED [$label | $profile]"
      status=1
      continue
    fi

    if timeout 600 cargo test "${args[@]}" "${profile_args[@]}" 2>&1 | tail -n 45; then
      echo "RESULT: PASS [$label | $profile]"
    else
      echo "RESULT: cargo test FAILED [$label | $profile]"
      status=1
    fi
  done
done

echo
if (( status == 0 )); then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$status"
