#!/usr/bin/env bash
# Full verification sweep: builds the C .so, enumerates every feature
# combination declared in Cargo.toml, and runs the whole differential suite in
# both the dev and release profiles for each one. Also re-derives the `nm -D`
# symbol diff outside of cargo, as an independent check of SYMBOLS.md.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
TIMEOUT=${TIMEOUT:-600}
fail=0

step() { printf '\n===== %s =====\n' "$*"; }

# --- 1. C shared object ----------------------------------------------------
step "building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "C  .so: $C_SO"

# --- 2. feature combinations ----------------------------------------------
# Every name under a [features] table, if any. Empty => only the default build.
FEATS=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' "$HERE/Cargo.toml")

COMBOS=("default")
if [ -n "$FEATS" ]; then
  COMBOS+=("none")
  for f in $FEATS; do COMBOS+=("$f"); done
  # all features together
  COMBOS+=("$(echo "$FEATS" | tr '\n' ',' | sed 's/,$//')")
else
  # No [features] table: `--no-default-features` is the same build, but run it
  # anyway so the claim in SYMBOLS.md/CONFIGS.md is actually exercised.
  COMBOS+=("none")
fi
echo "feature combinations: ${COMBOS[*]}"

flags_for() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    *)       echo "--no-default-features --features $1" ;;
  esac
}

# --- 3. build + test every (combo, profile) -------------------------------
for combo in "${COMBOS[@]}"; do
  FLAGS=$(flags_for "$combo")
  for profile in dev release; do
    PF=""; [ "$profile" = release ] && PF="--release"
    step "combo=$combo profile=$profile : cargo check"
    ( cd "$HERE" && timeout "$TIMEOUT" cargo check $FLAGS $PF --all-targets ) \
      || { echo "CHECK FAILED (combo=$combo profile=$profile)"; fail=1; continue; }

    step "combo=$combo profile=$profile : cargo build --lib (cdylib under test)"
    ( cd "$HERE" && timeout "$TIMEOUT" cargo build --lib $FLAGS $PF ) \
      || { echo "BUILD FAILED (combo=$combo profile=$profile)"; fail=1; continue; }

    step "combo=$combo profile=$profile : cargo test"
    ( cd "$HERE" && timeout "$TIMEOUT" cargo test $FLAGS $PF --tests ) \
      || { echo "TESTS FAILED (combo=$combo profile=$profile)"; fail=1; }
  done
done

# --- 4. independent nm -D symbol diff -------------------------------------
for profile in debug release; do
  R_SO="$HERE/target/$profile/libnormalize_lib.so"
  [ -f "$R_SO" ] || continue
  step "nm -D symbol diff (C vs Rust $profile)"
  nm -D --defined-only "$C_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > /tmp/_c_syms
  nm -D --defined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > /tmp/_r_syms
  MISSING=$(comm -23 /tmp/_c_syms /tmp/_r_syms)
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST ($profile):"; echo "$MISSING"; fail=1
  else
    echo "OK: 0 symbols missing from the Rust .so ($profile)"
    echo "C exports: $(tr '\n' ' ' < /tmp/_c_syms)"
  fi
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
