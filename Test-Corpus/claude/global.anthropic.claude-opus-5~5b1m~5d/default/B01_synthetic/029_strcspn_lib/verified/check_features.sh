#!/usr/bin/env bash
# Phase D driver: runs the full differential suite for EVERY feature combination
# and EVERY build profile of the Rust cdylib, plus the symbol diff for each.
#
# The crate declares no [features], so the feature power-set is {""} (the default
# = empty configuration). The loop is written generically anyway so that adding a
# feature automatically widens the matrix instead of silently leaving code paths
# unverified.
#
# Usage: ./check_features.sh
set -uo pipefail

cd "$(dirname "$0")"
CARGO_FLAGS="--offline"
FAILED=0
SUMMARY=()

note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# --- 0. the C shared object must exist -------------------------------------
C_SO="../c_src/build/libdriver.so"
if [[ ! -f $C_SO ]]; then
  note "building the C shared library"
  ( mkdir -p ../c_src/build && cd ../c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
fi

# --- 1. enumerate feature combinations (power-set of declared features) ----
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/=.*/,""); gsub(/ /,""); if ($0!="") print}' Cargo.toml
)
COMBOS=("")                                     # always test the default build
if (( ${#FEATURES[@]} > 0 )); then
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("$combo")
  done
fi
printf 'declared features : %s\n' "${FEATURES[*]:-<none>}"
printf 'combinations      : %s\n' "${#COMBOS[@]}"

# --- 2. for each combination x profile: build, diff symbols, run tests -----
for combo in "${COMBOS[@]}"; do
  if [[ -z $combo ]]; then
    FEAT_ARGS=(--no-default-features)
    label="no-default-features"
  else
    FEAT_ARGS=(--no-default-features --features "$combo")
    label="features=$combo"
  fi

  for profile in debug release; do
    [[ $profile == release ]] && PROF_ARGS=(--release) || PROF_ARGS=()
    tag="$label / $profile"

    note "building cdylib   [$tag]"
    if ! cargo build $CARGO_FLAGS "${PROF_ARGS[@]}" "${FEAT_ARGS[@]}" 2>&1 | tail -2; then
      fail "build [$tag]"; continue
    fi

    RUST_SO="target/$profile/libdriver.so"
    if [[ ! -f $RUST_SO ]]; then
      fail "cdylib not produced at $RUST_SO [$tag]"; continue
    fi

    # --- symbol diff: every C export must be exported by the Rust .so ---
    c_syms=$(nm -D --defined-only "$C_SO"   | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO"| awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -n $missing ]]; then
      fail "symbols missing from $RUST_SO [$tag]: $(echo "$missing" | tr '\n' ' ')"
    else
      echo "symbol diff empty ($(echo "$c_syms" | wc -l) C export(s)) [$tag]"
    fi

    # --- run every differential suite against THIS .so (once) ---
    note "running tests     [$tag]"
    log="target/difftest-${profile}-${combo:-default}.log"
    DRIVER_RUST_SO="$PWD/$RUST_SO" \
      timeout 600 cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}" -- --test-threads=1 \
      >"$log" 2>&1
    rc=$?
    grep -E '^(test result:|error|running [0-9])' "$log" | sed 's/^/    /'
    if (( rc == 0 )); then
      SUMMARY+=("PASS  $tag")
    else
      SUMMARY+=("FAIL  $tag  (see $log)")
      fail "tests [$tag] rc=$rc"
      grep -E '^(---- |thread .* panicked|assertion)' "$log" | head -20 | sed 's/^/    /'
    fi
  done
done

note "SUMMARY"
printf '%s\n' "${SUMMARY[@]}"
if (( FAILED )); then
  printf '\n\033[31mSOME CONFIGURATIONS FAILED\033[0m\n'; exit 1
fi
printf '\n\033[32mALL CONFIGURATIONS PASSED\033[0m\n'
