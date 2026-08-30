#!/usr/bin/env bash
# Enumerate every valid cargo feature combination and check + test each one.
#
# The feature list is read out of Cargo.toml rather than hardcoded, so this keeps
# working if features are added later.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- enumerate features from Cargo.toml -------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "" && a[1] != "default") print a[1] }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# All 2^n subsets, expressed as comma-separated --features arguments.
COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if ((mask & (1 << i))); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done
# Always cover the crate's own default feature set too.
COMBOS+=("__default__")

echo "combinations to verify: ${#COMBOS[@]}"

# --- build the C ground truth once ------------------------------------------
C_BUILD="../c_src/build"
if [[ ! -f "$C_BUILD/libdriver.so" ]]; then
  echo "==> building C shared library"
  mkdir -p "$C_BUILD" || exit 1
  (cd "$C_BUILD" &&
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    timeout 600 cmake --build . >/dev/null) || exit 1
fi

# --- check + build + test each combination ----------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  if [[ $combo == "__default__" ]]; then
    args=()
    label="(default features)"
  elif [[ -z $combo ]]; then
    # The empty subset: no features at all. `--features ''` is rejected by cargo.
    args=(--no-default-features)
    label="--no-default-features (no features)"
  else
    args=(--no-default-features --features "$combo")
    label="--no-default-features --features '${combo}'"
  fi

  # The test harness builds the cdylib itself (cargo test does not); it needs the
  # same feature flags so the loaded .so matches the compiled test binary.
  export DRIVER_CARGO_ARGS="${args[*]}"

  echo "=================================================================="
  echo "==> $label"

  if ! timeout 600 cargo check "${args[@]}" >/tmp/driver_check.log 2>&1; then
    echo "FAIL: cargo check $label"
    tail -n 40 /tmp/driver_check.log
    fail=1
    continue
  fi
  echo "    cargo check: ok"

  for profile in dev release; do
    prof_args=("${args[@]}")
    [[ $profile == release ]] && prof_args+=(--release)

    if ! timeout 600 cargo build "${prof_args[@]}" >/tmp/driver_build.log 2>&1; then
      echo "FAIL: cargo build ($profile) $label"
      tail -n 40 /tmp/driver_build.log
      fail=1
      continue
    fi
    if ! timeout 600 cargo test "${prof_args[@]}" >"/tmp/driver_test_${profile}.log" 2>&1; then
      echo "FAIL: cargo test ($profile) $label"
      tail -n 60 "/tmp/driver_test_${profile}.log"
      fail=1
      continue
    fi
    echo "    cargo test ($profile): ok"
  done
done

echo "=================================================================="
if ((fail)); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} combination(s) pass check + build + test (dev and release)"
