#!/usr/bin/env bash
# Phase A step 1 + Phase D: mechanically enumerate every valid feature
# combination from Cargo.toml, `cargo check` each one, then run the full
# differential suite (Phases B + C + D) under each.
#
# Usage: ./verify_all.sh
set -uo pipefail
cd "$(dirname "$0")"

# All scratch files go under $TMPDIR (the sandbox makes /tmp read-only).
WORK="${TMPDIR:-/tmp}/driver_verify.$$"
mkdir -p "$WORK" || { echo "cannot create scratch dir $WORK"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

FAIL=0
note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [PASS] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 0. Build the C reference library
# ---------------------------------------------------------------------------
note "Building C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/libdriver.so" || { bad "C build"; exit 1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations (powerset of [features] in Cargo.toml)
# ---------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
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

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  Cargo.toml declares NO [features]; the only configuration is the"
  echo "  empty/default feature set."
  COMBOS=("")
else
  echo "  optional features: ${FEATURES[*]}"
  COMBOS=()
  n=${#FEATURES[@]}
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  => ${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
# 2. cargo check every combination (must compile before anything else)
# ---------------------------------------------------------------------------
note "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  if timeout 600 cargo check --offline --no-default-features \
       ${combo:+--features "$combo"} >"$WORK/chk.log" 2>&1; then
    ok "cargo check --no-default-features --features '$label'"
  else
    bad "cargo check --features '$label'"; tail -30 "$WORK/chk.log"
  fi
  # Also check the test targets compile under this combo.
  if timeout 600 cargo check --offline --no-default-features \
       ${combo:+--features "$combo"} --all-targets >"$WORK/chkt.log" 2>&1; then
    ok "cargo check --all-targets --features '$label'"
  else
    bad "cargo check --all-targets --features '$label'"; tail -30 "$WORK/chkt.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Full differential suite (Phases B, C, D) per combination, per profile.
#    Release is included because `panic = "abort"` and optimisation apply only
#    there, and a consumer would ship that build.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  for profile in dev release; do
    note "Differential suite: features='$label' profile=$profile"
    relflag=""; [ "$profile" = release ] && relflag="--release"
    # The crate is cdylib-only, so integration tests do NOT link it and
    # `cargo test` alone will not build it. Build the .so explicitly first.
    if timeout 600 cargo build --offline --no-default-features \
         ${combo:+--features "$combo"} $relflag >"$WORK/b.log" 2>&1; then
      ok "built cdylib [features='$label' $profile]"
    else
      bad "cdylib build failed [features='$label' $profile]"; tail -30 "$WORK/b.log"
    fi
    if timeout 600 cargo test --offline --no-default-features \
         ${combo:+--features "$combo"} $relflag \
         -- --test-threads=1 >"$WORK/t.log" 2>&1; then
      ok "$(grep -c '^test .* ok$' "$WORK/t.log") tests passed  [features='$label' $profile]"
      grep -E '^test result:' "$WORK/t.log" | sed 's/^/    /'
    else
      bad "tests failed [features='$label' $profile]"
      grep -E '^(test .* FAILED|failures:|thread .* panicked|test result:)' "$WORK/t.log" | head -40
    fi
  done
done

note "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "  ALL CHECKS PASSED across ${#COMBOS[@]} feature combination(s) x 2 profiles."
else
  echo "  FAILURES PRESENT — see [FAIL] lines above."
fi
exit "$FAIL"
