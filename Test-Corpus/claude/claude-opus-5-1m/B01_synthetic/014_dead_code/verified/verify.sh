#!/usr/bin/env bash
# Runs the whole differential verification for EVERY valid feature combination.
#
# The feature list is read out of Cargo.toml rather than hard-coded, so the loop
# stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

# --- enumerate features declared in [features] (excluding `default`) ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# --- build the power set of feature combinations -----------------------------
COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then combo+="${FEATURES[i]},"; fi
  done
  COMBOS+=("${combo%,}")
done
# Always also verify the plain default build.
COMBOS+=("__default__")

status=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    args=""
    label="(default features)"
  elif [[ -z "$combo" ]]; then
    args="--no-default-features"
    label="--no-default-features (empty combination)"
  else
    args="--no-default-features --features $combo"
    label="--no-default-features --features $combo"
  fi

  echo
  echo "==================================================================="
  echo "== $label"
  echo "==================================================================="

  echo "-- cargo check --all-targets"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $CARGO_FLAGS --all-targets $args 2>&1 | tail -5; then
    echo "CHECK FAILED for $label"
    status=1
  fi

  echo "-- cargo test (differential, C .so vs Rust .so)"
  # The nested cdylib build inside the harness must use the same combination.
  # shellcheck disable=SC2086
  if ! CDIFF_CARGO_ARGS="$args" timeout 600 cargo test $CARGO_FLAGS $args --no-fail-fast 2>&1 |
    grep -E "^test [a-z]|test result|error|FAILED|panicked"; then
    echo "(no matching output)"
  fi
  # shellcheck disable=SC2086
  if ! CDIFF_CARGO_ARGS="$args" timeout 600 cargo test $CARGO_FLAGS $args --no-fail-fast >/dev/null 2>&1; then
    echo "TESTS FAILED for $label"
    status=1
  fi
done

# --- all-features build, for completeness -----------------------------------
echo
echo "==================================================================="
echo "== --all-features"
echo "==================================================================="
if ! timeout 600 cargo check $CARGO_FLAGS --all-targets --all-features 2>&1 | tail -3; then
  echo "CHECK FAILED for --all-features"
  status=1
fi
if ! CDIFF_CARGO_ARGS="--all-features" timeout 600 cargo test $CARGO_FLAGS --all-features --no-fail-fast 2>&1 |
  grep -E "test result|FAILED"; then
  echo "(no matching output)"
fi
if ! CDIFF_CARGO_ARGS="--all-features" timeout 600 cargo test $CARGO_FLAGS --all-features --no-fail-fast >/dev/null 2>&1; then
  echo "TESTS FAILED for --all-features"
  status=1
fi

# --- release profile (panic = "abort") builds and behaves identically -------
echo
echo "== release build sanity (profile.release panic = \"abort\") =="
if timeout 600 cargo build $CARGO_FLAGS --release >/dev/null 2>&1; then
  c_out=$(./c_src/build/driver 2>/dev/null || true)
  r_out=$(./target/release/driver 2>/dev/null || true)
  if [[ "$c_out" == "$r_out" ]]; then
    echo "release executable output matches the C executable"
  else
    echo "RELEASE OUTPUT MISMATCH"
    status=1
  fi
else
  echo "RELEASE BUILD FAILED"
  status=1
fi

echo
if ((status == 0)); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $status
