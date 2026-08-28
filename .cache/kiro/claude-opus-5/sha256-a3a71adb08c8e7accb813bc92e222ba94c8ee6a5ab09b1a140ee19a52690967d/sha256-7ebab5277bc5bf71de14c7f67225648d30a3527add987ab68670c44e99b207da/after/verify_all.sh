#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build-time
# configuration.
#
#   1. builds the C reference shared library
#   2. enumerates every feature combination declared in translation/Cargo.toml
#   3. `cargo check` + `cargo test` each combination, in dev and release
#   4. diffs `nm -D --defined-only` between the C and Rust shared objects
#
# Usage: ./verify_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_SRC="$ROOT/c_src"
RUST="$ROOT/translation"
LOG_DIR="/tmp/translation-verify"
mkdir -p "$LOG_DIR"

FAILURES=0
note()  { printf '\n=== %s ===\n' "$*"; }
fail()  { printf 'FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------- C reference
note "Building C reference library"
mkdir -p "$C_SRC/build"
if ! (cd "$C_SRC/build" \
      && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && timeout 600 cmake --build .) > "$LOG_DIR/cmake.log" 2>&1; then
  tail -20 "$LOG_DIR/cmake.log"
  echo "C build failed"; exit 1
fi
C_SO="$(find "$C_SRC/build" -name 'libdriver.so' -print -quit)"
echo "C library: $C_SO"

# ------------------------------------------------------- feature enumeration
# Every subset of the optional features declared under [features], excluding
# the "default" meta-feature. With no [features] table this yields a single
# empty combination (the only valid configuration).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=");
      gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
    }
  ' "$RUST/Cargo.toml"
)

COMBOS=("")   # the no-features configuration is always valid
if ((${#FEATURES[@]} > 0)); then
  count=$((1 << ${#FEATURES[@]}))
  for ((mask = 1; mask < count; mask++)); do
    combo=""
    for ((i = 0; i < ${#FEATURES[@]}; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

note "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  - ${c:-<no features>}"; done

# -------------------------------------------------------------- verification
for combo in "${COMBOS[@]}"; do
  label="${combo:-no-features}"
  slug="${label//,/+}"
  feat_args=(--no-default-features)
  [[ -n "$combo" ]] && feat_args+=(--features "$combo")

  note "cargo check [$label]"
  if timeout 600 cargo check --manifest-path "$RUST/Cargo.toml" "${feat_args[@]}" \
       > "$LOG_DIR/check-$slug.log" 2>&1; then
    echo "ok"
  else
    tail -30 "$LOG_DIR/check-$slug.log"; fail "cargo check [$label]"
  fi

  for profile in dev release; do
    prof_args=()
    [[ $profile == release ]] && prof_args=(--release)

    note "cargo test [$label / $profile]"
    if timeout 600 cargo test --manifest-path "$RUST/Cargo.toml" \
         "${feat_args[@]}" "${prof_args[@]}" \
         > "$LOG_DIR/test-$slug-$profile.log" 2>&1; then
      grep -E '^test result:' "$LOG_DIR/test-$slug-$profile.log"
    else
      tail -40 "$LOG_DIR/test-$slug-$profile.log"
      fail "cargo test [$label / $profile]"
    fi

    note "symbol parity [$label / $profile]"
    if ! timeout 600 cargo build --manifest-path "$RUST/Cargo.toml" \
           "${feat_args[@]}" "${prof_args[@]}" \
           > "$LOG_DIR/build-$slug-$profile.log" 2>&1; then
      tail -30 "$LOG_DIR/build-$slug-$profile.log"
      fail "cargo build [$label / $profile]"
      continue
    fi
    out_dir="$RUST/target/$([[ $profile == release ]] && echo release || echo debug)"
    RUST_SO="$out_dir/libdriver.so"

    syms() {
      nm -D --defined-only "$1" \
        | awk '$2 != "U" && NF >= 3 { print $3 }' \
        | grep -Ev '^(_init|_fini|__bss_start|_edata|_end)$' \
        | sort -u
    }
    missing="$(comm -23 <(syms "$C_SO") <(syms "$RUST_SO"))"
    if [[ -n "$missing" ]]; then
      echo "symbols exported by C but missing from Rust:"; echo "$missing"
      fail "symbol parity [$label / $profile]"
    else
      echo "ok ($(syms "$C_SO" | tr '\n' ' ')) "
    fi
  done
done

note "Summary"
if ((FAILURES == 0)); then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "$FAILURES failure(s); logs in $LOG_DIR"
fi
exit $((FAILURES > 0))
