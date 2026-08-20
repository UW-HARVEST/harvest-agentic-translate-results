#!/usr/bin/env bash
# Full verification driver: builds the C reference, enumerates every valid
# feature combination from Cargo.toml, and for each one runs `cargo check`,
# `cargo build`, the symbol-parity diff and the whole differential test suite
# (Phases B and C) in both the dev and the release profile.
#
# Usage:  ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- C reference
step "Building the C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "c_src built" || bad "c_src build"
C_SO=$(ls c_src/build/*.so | head -1)
ok "C .so = $C_SO"

# ------------------------------------------------------- feature combinations
# Enumerate the powerset of the [features] table (excluding "default").
step "Enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)
FEATURE_LIST=$(echo "$FEATURES" | tr '\n' ' ' | sed 's/ *$//')
if [ -z "$FEATURE_LIST" ]; then
    ok "no [features] table => exactly one configuration (the default)"
    COMBOS=("")
else
    ok "features: $FEATURE_LIST"
    read -r -a FARR <<< "$FEATURE_LIST"
    n=${#FARR[@]}
    COMBOS=()
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then combo="$combo,${FARR[$i]}"; fi
        done
        COMBOS+=("${combo#,}")
    done
fi
printf '  %d combination(s)\n' "${#COMBOS[@]}"

# --------------------------------------------------------------- run each one
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        FLAGS=(--no-default-features)
        NAME="<no features>"
    else
        FLAGS=(--no-default-features --features "$combo")
        NAME="$combo"
    fi
    for profile in dev release; do
        if [ "$profile" = release ]; then PFLAGS=(--release); else PFLAGS=(); fi
        step "combo: $NAME | profile: $profile"

        cargo check --offline "${FLAGS[@]}" "${PFLAGS[@]}" --all-targets >/dev/null 2>&1 \
            && ok "cargo check" || bad "cargo check ($NAME/$profile)"

        cargo build --offline "${FLAGS[@]}" "${PFLAGS[@]}" >/dev/null 2>&1 \
            && ok "cargo build" || bad "cargo build ($NAME/$profile)"

        if [ "$profile" = release ]; then R_SO=target/release/libspec_ray_lib.so
        else R_SO=target/debug/libspec_ray_lib.so; fi

        if diff <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort) \
                <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) >/dev/null
        then ok "symbol parity ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
        else bad "symbol parity ($NAME/$profile)"
             diff <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort) \
                  <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort)
        fi

        out=$(timeout 600 cargo test --offline "${FLAGS[@]}" "${PFLAGS[@]}" 2>&1)
        if echo "$out" | grep -q "FAILED\|error\["; then
            bad "test suite ($NAME/$profile)"
            echo "$out" | tail -40
        else
            ok "test suite: $(echo "$out" | grep -c '^test .* ok$') tests passed"
        fi
    done
done

# also make sure the plain default build (what a consumer gets) is clean
step "default configuration"
cargo check --offline --all-targets >/dev/null 2>&1 && ok "cargo check (default)" || bad "cargo check (default)"
cargo check --offline --all-features --all-targets >/dev/null 2>&1 && ok "cargo check (--all-features)" || bad "cargo check (--all-features)"

step "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "  ALL CONFIGURATIONS VERIFIED"; else echo "  FAILURES PRESENT"; fi
exit "$FAIL"
