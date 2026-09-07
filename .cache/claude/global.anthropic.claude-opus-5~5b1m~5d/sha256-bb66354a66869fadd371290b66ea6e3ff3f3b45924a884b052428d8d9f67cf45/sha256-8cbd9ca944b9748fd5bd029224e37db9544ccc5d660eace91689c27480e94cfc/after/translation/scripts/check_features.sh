#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY cargo feature
# combination, in BOTH the dev (overflow checks on) and release profiles.
#
# The cdylib must be built explicitly for each profile: `cargo test` alone does
# not build a `cdylib` target, and the tests load the `.so` through libloading.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

OFFLINE=""
if ! cargo search --limit 1 libloading >/dev/null 2>&1; then
  OFFLINE="--offline"
fi

# ---- enumerate feature combinations declared in Cargo.toml -----------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
  }
' Cargo.toml | grep -v '^default$')

echo "=== declared non-default features: [${FEATURES:-<none>}] ==="

# Build the combination list: always the default build and the
# --no-default-features build; plus the powerset of declared features.
COMBOS=()
COMBOS+=("DEFAULT")
COMBOS+=("NODEFAULT")
if [ -n "$FEATURES" ]; then
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("FEAT:$combo")
  done
fi

# ---- ensure the C library exists ------------------------------------------
C_BUILD=../c_src/build
if ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "building C shared library..."
  (mkdir -p "$C_BUILD" && cd "$C_BUILD" &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || {
    echo "C build FAILED"
    exit 1
  }
fi

FAIL=0
for profile in dev release; do
  if [ "$profile" = release ]; then PFLAG="--release"; else PFLAG=""; fi
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
    DEFAULT) FFLAGS=() ; label="default" ;;
    NODEFAULT) FFLAGS=(--no-default-features) ; label="no-default-features" ;;
    FEAT:*) FFLAGS=(--no-default-features --features "${combo#FEAT:}") ; label="features=${combo#FEAT:}" ;;
    esac

    echo
    echo "############################################################"
    echo "### profile=$profile  $label"
    echo "############################################################"

    # cdylib first (tests dlopen it), then the tests.
    if ! cargo build $OFFLINE $PFLAG "${FFLAGS[@]}" >/dev/null 2>&1; then
      echo "BUILD FAILED (profile=$profile $label)"
      FAIL=1
      continue
    fi
    if ! timeout 600 cargo test $OFFLINE $PFLAG "${FFLAGS[@]}" 2>&1 |
      grep -vE '^ *\{ 0x' | grep -E 'test result|FAILED|panicked'; then
      : # grep found nothing to print; fall through to the status check below
    fi
    # Re-run capturing status (cheap: everything is cached at this point).
    if ! timeout 600 cargo test $OFFLINE $PFLAG "${FFLAGS[@]}" >/dev/null 2>&1; then
      echo ">>> TESTS FAILED (profile=$profile $label)"
      FAIL=1
    else
      echo ">>> PASS (profile=$profile $label)"
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS x PROFILES PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAIL"
