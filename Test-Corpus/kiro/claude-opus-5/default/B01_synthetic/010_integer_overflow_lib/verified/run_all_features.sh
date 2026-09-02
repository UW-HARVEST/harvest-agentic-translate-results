#!/usr/bin/env bash
# Phase D: build both shared libraries and run the full differential suite under
# EVERY cargo feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are added later. `driver` currently declares no
# [features] table, so the enumeration yields exactly one combination: the
# default (which is identical to --no-default-features here).
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"

TIMEOUT="${TIMEOUT:-600}"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C ground truth
# --------------------------------------------------------------------------
step "Building C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/       { in_f = 1; next }
    /^\[/                 { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No optional features exist: the single configuration is the default one.
  COMBOS+=("<default>")
else
  COMBOS+=("<default>" "<none>")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

step "Feature combinations to verify (${#COMBOS[@]})"
printf '  %s\n' "${COMBOS[@]}"

# --------------------------------------------------------------------------
# 3. For each combination: check, build both profiles' cdylib, run the suite
#    against each profile's .so
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") FLAGS=() ;;
    "<none>")    FLAGS=(--no-default-features) ;;
    *)           FLAGS=(--no-default-features --features "$combo") ;;
  esac

  step "combo: $combo  (cargo ${FLAGS[*]:-})"

  if ! timeout "$TIMEOUT" cargo check "${FLAGS[@]}" >/dev/null 2>&1; then
    echo "  cargo check FAILED"; fail=1; continue
  fi
  echo "  cargo check ok"

  # `cargo test` does NOT build the cdylib for a cdylib-only crate, so build it
  # explicitly for both profiles.
  for profile in debug release; do
    if [ "$profile" = release ]; then BFLAGS=("${FLAGS[@]}" --release); else BFLAGS=("${FLAGS[@]}"); fi
    if ! timeout "$TIMEOUT" cargo build "${BFLAGS[@]}" >/dev/null 2>&1; then
      echo "  cargo build ($profile) FAILED"; fail=1; continue
    fi
    SO="$CRATE_DIR/target/$profile/libdriver.so"
    if [ ! -f "$SO" ]; then
      echo "  missing $SO"; fail=1; continue
    fi

    # Symbol parity for this exact artifact.
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$SO"   | awk '{print $3}' | sort -u))
    if [ -n "$missing" ]; then
      echo "  SYMBOL PARITY FAILED ($profile): missing -> $missing"; fail=1
    else
      echo "  symbol parity ok ($profile)"
    fi

    if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$SO" \
        timeout "$TIMEOUT" cargo test "${FLAGS[@]}" --test differential -q >/tmp/dt.$$ 2>&1; then
      echo "  differential ok ($profile): $(grep 'differential result' /tmp/dt.$$)"
    else
      echo "  differential FAILED ($profile)"; sed 's/^/    /' /tmp/dt.$$; fail=1
    fi
    rm -f /tmp/dt.$$
  done
done

step "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL feature combinations passed (check + build + symbol parity + differential, both profiles)."
else
  echo "FAILURES present."
fi
exit "$fail"
