#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination
# and BOTH cargo profiles. Feature names are extracted from Cargo.toml rather
# than hard-coded, so a newly added feature is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")"

C_BUILD=../c_src/build
if ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi

# --- enumerate declared features ------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of combinations to test: the default build, the
# no-default-features build, and every subset of the declared features
# (capped so the matrix stays bounded).
COMBOS=("DEFAULT" "NODEFAULT")
n=${#FEATURES[@]}
if (( n > 0 && n <= 10 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("FEATURES:$combo")
  done
elif (( n > 10 )); then
  for f in "${FEATURES[@]}"; do COMBOS+=("FEATURES:$f"); done
  COMBOS+=("FEATURES:$(IFS=,; echo "${FEATURES[*]}")")
fi

fail=0
for profile in debug release; do
  PROFILE_FLAG=""
  [[ $profile == release ]] && PROFILE_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      DEFAULT)   FLAGS=() ;;
      NODEFAULT) FLAGS=(--no-default-features) ;;
      FEATURES:*) FLAGS=(--no-default-features --features "${combo#FEATURES:}") ;;
    esac
    label="profile=$profile combo=$combo"
    echo "=================================================================="
    echo "== $label"
    echo "=================================================================="

    if ! timeout 600 cargo check $PROFILE_FLAG "${FLAGS[@]}" 2>&1 | tail -3; then
      echo "CHECK FAILED: $label"; fail=1; continue
    fi
    # The cdylib must exist for the tests to dlopen it.
    if ! timeout 600 cargo build $PROFILE_FLAG "${FLAGS[@]}" 2>&1 | tail -3; then
      echo "BUILD FAILED: $label"; fail=1; continue
    fi

    out=$(timeout 600 cargo test $PROFILE_FLAG "${FLAGS[@]}" 2>&1)
    echo "$out" | grep -E 'Running|test result|FAILED|panicked' || true
    if echo "$out" | grep -qE 'FAILED|error\[|error:'; then
      echo "TEST FAILED: $label"; fail=1
    fi

    # Symbol parity for this exact configuration.
    rust_so="target/$profile/libpremultiply_lib.so"
    c_so=$(ls "$C_BUILD"/lib*.so | head -1)
    diff_out=$(comm -23 \
      <(nm -D --defined-only --format=posix "$c_so" \
          | awk '$2 ~ /^[TDBRGSWV]$/ {print $1}' \
          | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_edata|_end|__bss_start|_init|_fini)' \
          | sort -u) \
      <(nm -D --defined-only --format=posix "$rust_so" \
          | awk '$2 ~ /^[TDBRGSWV]$/ {print $1}' \
          | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_edata|_end|__bss_start|_init|_fini)' \
          | sort -u))
    if [[ -n "$diff_out" ]]; then
      echo "SYMBOL DIFF NON-EMPTY ($label): $diff_out"; fail=1
    else
      echo "symbol diff: EMPTY (ok)  [$label]"
    fi
  done
done

echo "=================================================================="
if (( fail )); then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: all combinations x profiles PASSED"
