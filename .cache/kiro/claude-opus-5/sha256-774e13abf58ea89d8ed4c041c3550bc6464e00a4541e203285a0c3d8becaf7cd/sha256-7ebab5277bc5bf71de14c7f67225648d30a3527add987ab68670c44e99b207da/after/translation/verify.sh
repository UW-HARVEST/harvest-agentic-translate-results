#!/usr/bin/env bash
# Differential verification of the Rust translation against the C ground truth.
#
# 1. enumerates every valid Cargo feature combination
# 2. `cargo check` for each
# 3. builds the C shared library
# 4. builds the Rust cdylib and runs the libloading-based tests for each
#    combination, against both the release and the debug Rust .so
#
# All cargo/cmake invocations are wrapped in `timeout`.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SRC="$ROOT/c_src"
RUST="$ROOT/translation"
TIMEOUT=${TIMEOUT:-600}
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# --- 1. enumerate feature combinations ---------------------------------------
# Features are read from the [features] table of Cargo.toml. `default` is
# excluded from the powerset (it is covered by the empty combination plus
# --no-default-features runs of its members).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "="); gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1]
    }
  ' "$RUST/Cargo.toml"
)

COMBOS=("")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

step "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do printf '  [%s]\n' "${c:-<none>}"; done

# --- 2. cargo check every combination ----------------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo check --no-default-features --features '${combo:-<none>}'"
  if ! (cd "$RUST" && timeout "$TIMEOUT" cargo check --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | tail -5); then
    fail "cargo check failed for [${combo:-<none>}]"
  fi
done

# --- 3. build the C shared library -------------------------------------------
step "build C shared library"
mkdir -p "$C_SRC/build"
if ! (cd "$C_SRC/build" \
      && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && timeout "$TIMEOUT" cmake --build . 2>&1 | tail -3); then
  fail "C build failed"
fi
C_SO="$C_SRC/build/libdriver.so"
[[ -f "$C_SO" ]] || fail "missing $C_SO"

# --- 4. per-combination build, symbol diff and differential tests ------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"

  for profile in release debug; do
    step "features [$label] profile $profile"

    build_args=(--no-default-features)
    [[ -n "$combo" ]] && build_args+=(--features "$combo")
    [[ "$profile" == release ]] && build_args+=(--release)

    if ! (cd "$RUST" && timeout "$TIMEOUT" cargo build "${build_args[@]}" 2>&1 | tail -3); then
      fail "cargo build failed for [$label]/$profile"
      continue
    fi

    RUST_SO="$RUST/target/$profile/libdriver.so"
    if [[ ! -f "$RUST_SO" ]]; then
      fail "missing $RUST_SO"
      continue
    fi

    printf -- '--- exported symbols ---\n'
    printf 'C   : %s\n' "$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort | tr '\n' ' ')"
    printf 'Rust: %s\n' "$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' \
                            | grep -Ev '^(_ZN|_R|__rust|rust_)' | sort | tr '\n' ' ')"

    if ! (cd "$RUST" && C_DRIVER_SO="$C_SO" RUST_DRIVER_SO="$RUST_SO" VERIFY_FEATURES="$combo" \
          timeout "$TIMEOUT" cargo test "${build_args[@]}" 2>&1 \
          | grep -E 'Running|test result|^test |panicked|FAILED'); then
      fail "cargo test failed for [$label]/$profile"
    fi
  done
done

step "summary"
if ((FAILED)); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "ALL FEATURE COMBINATIONS PASS"
