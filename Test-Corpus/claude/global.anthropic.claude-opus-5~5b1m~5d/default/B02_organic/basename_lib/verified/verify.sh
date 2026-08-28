#!/usr/bin/env bash
# Full differential verification: builds the C .so, builds the Rust cdylib in
# every profile/feature combination, and runs the whole test suite against each.
#
# `cargo test` does NOT rebuild a cdylib-only lib target, so the explicit
# `cargo build` before each `cargo test` is mandatory, not cosmetic. The
# staleness guard in tests/common/mod.rs enforces it.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="--offline"   # crates.io is unreachable in this sandbox
FAILED=0
LOGDIR="$CRATE_DIR/target/verify-logs"
mkdir -p "$LOGDIR"

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=1; }

# --------------------------------------------------------------- C shared lib
step "Building the C ground-truth shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { fail "missing $C_SO"; exit 1; }
echo "  -> $C_SO"

# ------------------------------------------------------- feature combinations
# Enumerate feature combinations mechanically from Cargo.toml rather than
# hard-coding them, so a newly added feature cannot be silently skipped.
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  no [features] section -> the only combination is the default (empty) one"
  COMBOS=("DEFAULT")
else
  echo "  features: ${FEATURES[*]}"
  COMBOS=("DEFAULT" "NONE")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------- run the matrix
run_matrix() {
  local profile_flag="$1" profile_dir="$2" combo="$3"
  local feat_flags=()
  case "$combo" in
    DEFAULT) feat_flags=() ;;
    NONE)    feat_flags=(--no-default-features) ;;
    *)       feat_flags=(--no-default-features --features "$combo") ;;
  esac

  local label="profile=${profile_dir} features=${combo}"
  step "$label"

  # 1. build the cdylib under test
  if ! cargo build $CARGO_FLAGS $profile_flag "${feat_flags[@]}" \
        > "$LOGDIR/build-${profile_dir}-${combo//,/_}.log" 2>&1; then
    fail "cargo build ($label)"; tail -20 "$LOGDIR/build-${profile_dir}-${combo//,/_}.log"; return
  fi
  local rust_so="$CRATE_DIR/target/${profile_dir}/libdriver.so"
  [ -f "$rust_so" ] || { fail "missing $rust_so ($label)"; return; }

  # 2. symbol parity against the C .so
  local missing
  missing="$(comm -23 \
    <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))"
  if [ -n "$missing" ]; then
    fail "symbols exported by C but not Rust ($label): $(echo "$missing" | tr '\n' ' ')"
  else
    echo "  symbol parity: OK ($(nm -D --defined-only "$rust_so" | wc -l) exported)"
  fi

  # 3. the differential suite, pointed at this exact pair of .so files.
  #    NOTE: the test harness itself is always built with the dev profile,
  #    because [profile.release] sets panic="abort", which libtest cannot use.
  #    DRIVER_RUST_SO makes the dev-profile harness load the release cdylib.
  if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$rust_so" \
       cargo test $CARGO_FLAGS "${feat_flags[@]}" \
       > "$LOGDIR/test-${profile_dir}-${combo//,/_}.log" 2>&1; then
    grep -E '^test result:' "$LOGDIR/test-${profile_dir}-${combo//,/_}.log" | sed 's/^/  /'
  else
    fail "cargo test ($label)"
    grep -E '^(test result:|---- |thread .* panicked|assertion)' \
      "$LOGDIR/test-${profile_dir}-${combo//,/_}.log" | head -40 | sed 's/^/  /'
  fi
}

for combo in "${COMBOS[@]}"; do
  run_matrix ""          "debug"   "$combo"
  run_matrix "--release" "release" "$combo"
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo -e "\033[32mALL CHECKS PASSED\033[0m"
else
  echo -e "\033[31mTHERE WERE FAILURES\033[0m"
fi
exit "$FAILED"
