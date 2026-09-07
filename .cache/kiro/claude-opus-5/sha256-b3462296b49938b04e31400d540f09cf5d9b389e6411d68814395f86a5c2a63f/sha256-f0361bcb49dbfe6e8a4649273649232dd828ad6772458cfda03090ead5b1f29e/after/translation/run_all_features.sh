#!/usr/bin/env bash
# Phase D driver: run the full differential suite under EVERY feature
# combination and under both profiles, then re-check symbol parity.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAILED=0

# ---- build the C reference library -----------------------------------------
if [ ! -f "$ROOT/c_src/build/libdriver.so" ]; then
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi

# ---- enumerate feature combinations ----------------------------------------
# Every feature name declared in [features], excluding `default`.
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("--default")                       # default features
COMBOS+=("--none")                          # --no-default-features
if [ -n "$FEATURES" ]; then
  # power set of the declared features, each with --no-default-features
  n=$(echo "$FEATURES" | wc -l)
  names=($FEATURES)
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then combo="$combo,${names[$b]}"; fi
    done
    COMBOS+=("${combo#,}")
  done
  COMBOS+=("--all")
fi

echo "feature combinations to verify: ${#COMBOS[@]} -> ${COMBOS[*]}"
[ -z "$FEATURES" ] && echo "(Cargo.toml declares no [features]; default == --no-default-features)"

# ---- run the suite for each (combo, profile) -------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    --default) FLAGS=() ;;
    --none)    FLAGS=(--no-default-features) ;;
    --all)     FLAGS=(--all-features) ;;
    *)         FLAGS=(--no-default-features --features "$combo") ;;
  esac

  for profile in debug release; do
    PFLAGS=()
    [ "$profile" = release ] && PFLAGS=(--release)

    echo
    echo "=============================================================="
    echo "features=$combo profile=$profile"
    echo "=============================================================="

    if ! timeout 600 cargo build "${FLAGS[@]}" "${PFLAGS[@]}" 2>&1 | tail -3; then
      echo "BUILD FAILED (features=$combo profile=$profile)"; FAILED=1; continue
    fi

    # symbol parity for this exact artifact
    C_SYMS=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" \
              | awk '$2=="T"{print $3}' | sort)
    R_SYMS=$(nm -D --defined-only "target/$profile/libdriver.so" \
              | awk '$2=="T"{print $3}' | sort)
    MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED (features=$combo profile=$profile); missing:"
      echo "$MISSING"; FAILED=1
    else
      echo "symbol parity OK ($(echo "$C_SYMS" | wc -l) C symbols, 0 missing)"
    fi

    if ! timeout 600 cargo test --test differential "${FLAGS[@]}" "${PFLAGS[@]}" 2>&1 \
         | grep -E 'checks passed|FAILURES:|all differential|^FAIL '; then
      echo "TEST RUN FAILED (features=$combo profile=$profile)"; FAILED=1
    fi
  done
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
