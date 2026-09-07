#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination and
# under both codegen profiles.  Feature names are read out of Cargo.toml, so this
# stays correct if features are added later.
set -uo pipefail
cd "$(dirname "$0")"

OFFLINE=${OFFLINE:---offline}

# --- 1. rebuild the C shared object -----------------------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
echo "C .so: $(ls ../c_src/build/*.so)"

# --- 2. enumerate the declared features -------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "declared non-default features (${N}): ${FEATURES[*]:-<none>}"

# every subset of the feature set, as a comma list
COMBOS=()
for ((m = 0; m < (1 << N); m++)); do
  sel=""
  for ((b = 0; b < N; b++)); do
    if (( m & (1 << b) )); then sel="${sel:+$sel,}${FEATURES[b]}"; fi
  done
  COMBOS+=("$sel")
done

FAIL=0
run() { # run <label> <profile> <extra cargo args...>
  local label=$1 profile=$2; shift 2
  local pflag=(); [[ $profile == release ]] && pflag=(--release)
  echo
  echo "=================================================================="
  echo "== $label"
  echo "=================================================================="
  if ! DIFFTEST_RUST_PROFILE=$profile timeout 600 cargo test $OFFLINE "${pflag[@]}" "$@" 2>&1 \
       | grep -E 'test result|^error|FAILED|panicked' ; then :; fi
  # shellcheck disable=SC2181
  if ! DIFFTEST_RUST_PROFILE=$profile timeout 600 cargo test $OFFLINE "${pflag[@]}" "$@" >/dev/null 2>&1; then
    echo ">>> $label FAILED"
    FAIL=1
  else
    echo ">>> $label OK"
  fi
}

# --- 3. default features, both profiles -------------------------------------
run "default features / release" release
run "default features / debug"   debug

# --- 4. --no-default-features + every subset --------------------------------
for combo in "${COMBOS[@]}"; do
  if [[ -z $combo ]]; then
    run "--no-default-features (empty combo) / release" release --no-default-features
    run "--no-default-features (empty combo) / debug"   debug   --no-default-features
  else
    run "--no-default-features --features $combo / release" release --no-default-features --features "$combo"
    run "--no-default-features --features $combo / debug"   debug   --no-default-features --features "$combo"
  fi
done

# --- 5. all features at once -------------------------------------------------
run "--all-features / release" release --all-features
run "--all-features / debug"   debug   --all-features

echo
if [[ $FAIL -eq 0 ]]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $FAIL
