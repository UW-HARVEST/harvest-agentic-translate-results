#!/usr/bin/env bash
# Run the full differential suite under every buildable feature combination and
# both optimisation profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -uo pipefail

cd "$(dirname "$0")"
FAILED=0

# --- 1. Make sure the C reference library exists ---------------------------
C_SO=../c_src/build/libdriver.so
if [[ ! -f $C_SO ]]; then
  echo "== building the C reference library =="
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) || { echo "C build FAILED"; exit 1; }
fi

# --- 2. Enumerate declared features --------------------------------------
# Everything between a `[features]` header and the next `[section]`, minus
# `default`.
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

echo "== declared features: ${#FEATURES[@]} ${FEATURES[*]-} =="

# Build the list of --features arguments: the empty set, every single feature,
# and (when there are few enough) the full power set.
COMBOS=()
N=${#FEATURES[@]}
if (( N == 0 )); then
  COMBOS=("")
elif (( N <= 12 )); then
  for (( mask=0; mask < (1<<N); mask++ )); do
    combo=""
    for (( i=0; i<N; i++ )); do
      if (( mask & (1<<i) )); then
        combo+="${combo:+,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
else
  COMBOS+=("")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  COMBOS+=("$(IFS=,; echo "${FEATURES[*]}")")
fi

run_one() {
  local desc="$1"; shift
  echo
  echo "=================================================================="
  echo "== $desc"
  echo "=================================================================="
  if ! timeout 600 cargo test "$@" 2>&1 | tail -n 60; then
    echo ">>> FAILED: $desc"
    FAILED=1
  fi
}

# --- 3. cargo check every combination first (fast fail) -------------------
for combo in "${COMBOS[@]}"; do
  if [[ -z $combo ]]; then
    echo "-- check: --no-default-features"
    timeout 300 cargo check --no-default-features >/dev/null 2>&1 \
      || { echo ">>> check FAILED for --no-default-features"; FAILED=1; }
    echo "-- check: (default features)"
    timeout 300 cargo check >/dev/null 2>&1 \
      || { echo ">>> check FAILED for default"; FAILED=1; }
  else
    echo "-- check: --no-default-features --features $combo"
    timeout 300 cargo check --no-default-features --features "$combo" >/dev/null 2>&1 \
      || { echo ">>> check FAILED for $combo"; FAILED=1; }
  fi
done

# --- 4. Full test run per combination, in dev AND release -----------------
# The release profile is a genuinely different configuration here: it sets
# `panic = "abort"` and enables optimisation, which changes how the raw pointer
# reads in `print_foo` are code-generated.
for profile_flag in "" "--release"; do
  pdesc=${profile_flag:-dev}
  for combo in "${COMBOS[@]}"; do
    if [[ -z $combo ]]; then
      run_one "profile=$pdesc features=(default)" ${profile_flag:+$profile_flag}
      run_one "profile=$pdesc features=(none)" ${profile_flag:+$profile_flag} --no-default-features
    else
      run_one "profile=$pdesc features=$combo" ${profile_flag:+$profile_flag} \
        --no-default-features --features "$combo"
    fi
  done
done

# --- 5. Symbol diff, printed explicitly for the record --------------------
echo
echo "=================================================================="
echo "== nm -D symbol diff (C vs Rust), both profiles"
echo "=================================================================="
for p in debug release; do
  RS=target/$p/libdriver.so
  [[ -f $RS ]] || { echo "-- $p: no libdriver.so, skipped"; continue; }
  diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$RS"  | awk '{print $3}' | sort) \
    > /tmp/symdiff-$p.txt
  # Only symbols missing from Rust ("<" lines) are failures.
  MISSING=$(grep '^<' /tmp/symdiff-$p.txt | sed 's/^< //' || true)
  if [[ -n $MISSING ]]; then
    echo "-- $p: MISSING FROM RUST:"; echo "$MISSING"; FAILED=1
  else
    echo "-- $p: 0 symbols missing from the Rust .so"
  fi
done

echo
if (( FAILED )); then
  echo "############ SOME CONFIGURATIONS FAILED ############"
  exit 1
fi
echo "############ ALL CONFIGURATIONS PASSED ############"
