#!/usr/bin/env bash
# Run the whole differential suite under every build configuration:
#   * every feature combination declared in Cargo.toml (currently: none, so the
#     default / --no-default-features / --all-features cases all coincide)
#   * both cargo profiles (dev and release; the release profile sets
#     panic = "abort" and optimises, which is a genuinely different .so)
#
# Usage:  ./run_all_configs.sh
set -uo pipefail
cd "$(dirname "$0")"

fail=0

# ---------------------------------------------------------------- build the C reference
if [ ! -f ../c_src/build/lib*.so ]; then
  (cd ../c_src && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
ls ../c_src/build/lib*.so >/dev/null || { echo "no C .so"; exit 1; }

# ---------------------------------------------------------------- enumerate features
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ ${#FEATURES[@]} -eq 0 ]; then
  COMBOS+=("")                       # default (== no features exist)
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((m = 0; m < total; m++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (( (m >> b) & 1 )) && sel+=("${FEATURES[$b]}")
    done
    if [ ${#sel[@]} -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("")            # plain default feature set
  COMBOS+=("--all-features")
fi

# ---------------------------------------------------------------- run
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test --offline ${profile:-<dev>} ${combo:-<default-features>}"
    echo "=============================================================="
    echo ">>> $label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    # the private-helper probe cdylib must be current: `cargo test --test X`
    # does not rebuild examples, and tests/private.rs refuses a stale one
    timeout 600 cargo build --offline $profile $combo --example private_probe >/dev/null 2>&1
    # shellcheck disable=SC2086
    if timeout 600 cargo test --offline $profile $combo 2>&1 | tail -n 25; then
      echo "PASS: $label"
    else
      echo "FAIL: $label"
      fail=1
    fi
  done
done

echo
if [ $fail -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $fail
