#!/usr/bin/env bash
# Enumerate and verify every build configuration of this crate.
#
# `Cargo.toml` declares no `[features]` and `c_src/CMakeLists.txt` declares no
# options or `#ifdef`s, so the feature power set is the single empty combination.
# The remaining configuration axes that can genuinely change generated code are
# the cargo profile (`debug` has overflow checks on, `release` sets
# `panic = "abort"` and optimises) and the workspace member set.
#
# Usage: ./check_all_configs.sh [--tests]
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=(--offline)
RUN_TESTS=0
[[ "${1:-}" == "--tests" ]] && RUN_TESTS=1

fail=0
step() {
  local desc="$1"; shift
  echo "=============================================================="
  echo ">>> $desc"
  echo "    \$ $*"
  if "$@"; then
    echo "    OK: $desc"
  else
    echo "    FAIL: $desc"
    fail=1
  fi
}

# ---------------------------------------------------------------------------
# Feature combinations. Derived mechanically from Cargo.toml so that the loop
# keeps working if features are ever added.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      print
    }
  ' Cargo.toml
)
echo "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Power set of FEATURES (empty for this crate -> exactly one combination: "").
COMBOS=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done
echo "feature combinations to verify: ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  step "cargo check --no-default-features --features '$combo' (workspace)" \
    cargo check "${CARGO_FLAGS[@]}" --workspace --all-targets \
    --no-default-features --features "$combo"
  step "cargo check --release --no-default-features --features '$combo'" \
    cargo check "${CARGO_FLAGS[@]}" --workspace --all-targets --release \
    --no-default-features --features "$combo"
done

step "cargo check --all-features (workspace, all targets)" \
  cargo check "${CARGO_FLAGS[@]}" --workspace --all-targets --all-features

step "cargo build (debug, workspace)" \
  cargo build "${CARGO_FLAGS[@]}" --workspace
step "cargo build (release, workspace)" \
  cargo build "${CARGO_FLAGS[@]}" --workspace --release

step "zero-warning build of every target (debug)" \
  env RUSTFLAGS=-Dwarnings cargo test "${CARGO_FLAGS[@]}" --workspace --all-targets --no-run
step "zero-warning build of every target (release)" \
  env RUSTFLAGS=-Dwarnings cargo build "${CARGO_FLAGS[@]}" --workspace --release

if (( RUN_TESTS )); then
  for combo in "${COMBOS[@]}"; do
    step "cargo test --no-default-features --features '${combo:-<none>}' (debug)" \
      cargo test "${CARGO_FLAGS[@]}" --workspace --no-default-features --features "$combo" --no-fail-fast
    step "cargo test --release --no-default-features --features '${combo:-<none>}'" \
      cargo test "${CARGO_FLAGS[@]}" --workspace --release --no-default-features --features "$combo" --no-fail-fast
  done
  step "cargo test --all-features (debug)" \
    cargo test "${CARGO_FLAGS[@]}" --workspace --all-features --no-fail-fast
fi

echo "=============================================================="
if (( fail )); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all configurations OK"
