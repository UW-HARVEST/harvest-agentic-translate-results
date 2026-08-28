#!/usr/bin/env bash
# Differential verification of translation/ against c_src/.
#
#  1. enumerate every feature combination declared in Cargo.toml
#  2. cargo check each one
#  3. build the C shared library
#  4. cargo test each combination (tests load both .so files via libloading)
#  5. compare exported symbols, for the debug and release Rust artifacts
#
# Usage: ./verify_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
LOGDIR="${TMPDIR:-/tmp}/driver_verify"
mkdir -p "$LOGDIR"
TIMEOUT=600
fail=0

note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ ok ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations
# ---------------------------------------------------------------------------
note "Feature combinations declared in Cargo.toml"

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$HERE/Cargo.toml"
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  no [features] table: the only configuration is the default one"
  COMBOS=("")
else
  echo "  features: ${FEATURES[*]}"
  # Power set of the declared features.
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
  printf '  %d combination(s)\n' "${#COMBOS[@]}"
fi

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
note "cargo check"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  log="$LOGDIR/check_${combo//,/_}.log"
  if timeout "$TIMEOUT" cargo check --manifest-path "$HERE/Cargo.toml" \
        --all-targets --no-default-features \
        ${combo:+--features "$combo"} > "$log" 2>&1; then
    ok "check $label"
  else
    bad "check $label (see $log)"
    tail -20 "$log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Build the C shared library
# ---------------------------------------------------------------------------
note "Build the C reference library"
(
  cd "$ROOT/c_src" &&
  mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
  cmake --build .
) > "$LOGDIR/c_build.log" 2>&1 && ok "c_src built" || {
  bad "c_src build failed (see $LOGDIR/c_build.log)"; tail -20 "$LOGDIR/c_build.log";
}
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] && ok "$C_SO" || bad "missing $C_SO"

# ---------------------------------------------------------------------------
# 4. cargo test for every combination
# ---------------------------------------------------------------------------
note "cargo test (differential, both .so loaded via libloading)"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  log="$LOGDIR/test_${combo//,/_}.log"
  if timeout "$TIMEOUT" cargo test --manifest-path "$HERE/Cargo.toml" \
        --no-default-features ${combo:+--features "$combo"} \
        > "$log" 2>&1; then
    ok "test $label ($(grep -c '^test .* ok$' "$log") passing assertions groups)"
  else
    bad "test $label (see $log)"
    grep -E "^(test |failures|assertion|error)" "$log" | head -40
  fi
done

# ---------------------------------------------------------------------------
# 5. Symbol parity, for both profiles
# ---------------------------------------------------------------------------
note "Exported-symbol parity"
syms() { nm -D --defined-only "$1" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u; }

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  for profile in debug release; do
    build_args=(--manifest-path "$HERE/Cargo.toml" --no-default-features)
    [ -n "$combo" ] && build_args+=(--features "$combo")
    [ "$profile" = release ] && build_args+=(--release)
    if ! timeout "$TIMEOUT" cargo build "${build_args[@]}" \
          > "$LOGDIR/build_${profile}_${combo//,/_}.log" 2>&1; then
      bad "build $profile $label"
      continue
    fi
    R_SO="$HERE/target/$profile/libdriver.so"
    if [ ! -f "$R_SO" ]; then
      bad "missing $R_SO"
      continue
    fi
    missing="$(comm -23 <(syms "$C_SO") <(syms "$R_SO"))"
    if [ -z "$missing" ]; then
      ok "symbols $profile $label ($(syms "$C_SO" | tr '\n' ' '))"
    else
      bad "symbols $profile $label — missing: $(echo "$missing" | tr '\n' ' ')"
    fi

    # Re-run the differential suite against this exact artifact.
    log="$LOGDIR/test_${profile}_${combo//,/_}.log"
    if DRIVER_RUST_SO="$R_SO" timeout "$TIMEOUT" cargo test \
          --manifest-path "$HERE/Cargo.toml" --no-default-features \
          ${combo:+--features "$combo"} > "$log" 2>&1; then
      ok "differential vs $profile artifact, $label"
    else
      bad "differential vs $profile artifact, $label (see $log)"
      grep -E "^(test |failures|assertion)" "$log" | head -40
    fi
  done
done

note "Result"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
