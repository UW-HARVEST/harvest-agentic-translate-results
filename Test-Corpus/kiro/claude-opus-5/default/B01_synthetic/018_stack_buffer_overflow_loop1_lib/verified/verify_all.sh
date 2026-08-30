#!/usr/bin/env bash
# Verify the translation against the C library for every build-time
# configuration: each Cargo feature combination, in both debug and release.
set -uo pipefail
cd "$(dirname "$0")"

TIMEOUT=${TIMEOUT:-600}
fail=0

# --- enumerate feature combinations from Cargo.toml ---------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=("")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "features declared: ${#FEATURES[@]} -> ${FEATURES[*]:-<none>}"
echo "combinations to verify: ${#COMBOS[@]}"

# --- build the C reference library -------------------------------------------
(
  cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FAIL: C library build"; exit 1; }
echo "C reference library built"

run() { # label, then command
  local label="$1"; shift
  if timeout "$TIMEOUT" "$@" >/tmp/driver_verify.log 2>&1; then
    echo "PASS  $label"
  else
    echo "FAIL  $label"
    tail -n 30 /tmp/driver_verify.log
    fail=1
  fi
}

for combo in "${COMBOS[@]}"; do
  if [[ -z $combo ]]; then
    flags=(--no-default-features)
    name="<no features>"
  else
    flags=(--no-default-features --features "$combo")
    name="$combo"
  fi

  run "check   [$name]" cargo check "${flags[@]}" --all-targets
  for profile in debug release; do
    if [[ $profile == release ]]; then rel=(--release); else rel=(); fi
    run "build   [$name/$profile]" cargo build "${flags[@]}" "${rel[@]}"
    run "symbols+tests [$name/$profile]" cargo test "${flags[@]}" "${rel[@]}"
  done
done

# default feature set as a user would get it
run "check   [default]" cargo check --all-targets
run "tests   [default/debug]" cargo test
run "tests   [default/release]" cargo test --release

if ((fail)); then echo "=== VERIFICATION FAILED ==="; exit 1; fi
echo "=== ALL CONFIGURATIONS VERIFIED ==="
