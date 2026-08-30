#!/usr/bin/env bash
# Differential verification driver: builds the C reference library, enumerates
# every valid Cargo feature combination, then runs `cargo check` and
# `cargo test` for each one and compares exported dynamic symbols.
#
# Usage: translation/verify.sh [--release]
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/translation"
C_BUILD_DIR="$REPO_ROOT/c_src/build"
TIMEOUT=600

PROFILE_ARGS=()
PROFILE_DIR="debug"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_ARGS=(--release)
  PROFILE_DIR="release"
fi

fail=0
note() { printf '\n== %s\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. C reference library (default CMake configuration)
# ---------------------------------------------------------------------------
note "Building C reference library"
mkdir -p "$C_BUILD_DIR"
(
  cd "$C_BUILD_DIR" &&
    timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >cmake.log 2>&1 &&
    timeout $TIMEOUT cmake --build . >build.log 2>&1
) || {
  echo "FAIL: C build failed; see $C_BUILD_DIR/{cmake,build}.log"
  exit 1
}
C_SO="$C_BUILD_DIR/libdriver.so"
echo "ok: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from [features] in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=")
      gsub(/[[:space:]]/, "", kv[1])
      if (kv[1] != "default") print kv[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  # No declared features: the empty set is the only configuration.
  COMBOS=("")
else
  total=$((1 << ${#FEATURES[@]}))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < ${#FEATURES[@]}; i++)); do
      if ((mask & (1 << i))); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

note "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do
  echo "  - '${c:-<none>}'"
done

# ---------------------------------------------------------------------------
# 3. check / build / test / symbol-compare per combination
# ---------------------------------------------------------------------------
cd "$CRATE_DIR"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  # Let the test harness build a matching cdylib if it ever needs to.
  printf '%s' "${args[*]}" >tests/feature_flags.txt

  note "features='$label' :: cargo check"
  if ! timeout $TIMEOUT cargo check "${args[@]}" "${PROFILE_ARGS[@]}" --all-targets \
    >/tmp/check-$$.log 2>&1; then
    echo "FAIL: cargo check (features='$label')"
    tail -30 /tmp/check-$$.log
    fail=1
    continue
  fi
  echo "ok"

  note "features='$label' :: cargo build --lib"
  if ! timeout $TIMEOUT cargo build --lib "${args[@]}" "${PROFILE_ARGS[@]}" \
    >/tmp/build-$$.log 2>&1; then
    echo "FAIL: cargo build (features='$label')"
    tail -30 /tmp/build-$$.log
    fail=1
    continue
  fi
  RUST_SO="$CRATE_DIR/target/$PROFILE_DIR/libdriver.so"
  echo "ok: $RUST_SO"

  note "features='$label' :: exported symbol comparison (nm -D)"
  # Defined symbols only (uppercase type letter), minus toolchain runtime hooks.
  strip_runtime='^(_init|_fini|_edata|_end|__bss_start|__gmon_start__)$|^(_ITM_|__cxa_|__rust|rust_|_Z|_R)'
  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | grep -Ev "$strip_runtime" | sort -u)
  rust_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$rust_syms"))
  if [[ -n "$missing" ]]; then
    echo "FAIL: Rust .so is missing exports present in the C .so:"
    printf '  %s\n' $missing
    fail=1
  else
    echo "ok: all $(printf '%s\n' "$c_syms" | wc -l) C exports present in the Rust .so"
    printf '  %s\n' $c_syms
  fi

  note "features='$label' :: cargo test"
  if ! timeout $TIMEOUT cargo test "${args[@]}" "${PROFILE_ARGS[@]}" -- --test-threads=1 \
    >/tmp/test-$$.log 2>&1; then
    echo "FAIL: cargo test (features='$label')"
    tail -60 /tmp/test-$$.log
    fail=1
  else
    grep -E '^test result:' /tmp/test-$$.log
    echo "ok"
  fi
done

rm -f tests/feature_flags.txt /tmp/check-$$.log /tmp/build-$$.log /tmp/test-$$.log

note "RESULT"
if [[ $fail -eq 0 ]]; then
  echo "PASS: every feature combination matches the C reference"
else
  echo "FAIL: see above"
fi
exit $fail
