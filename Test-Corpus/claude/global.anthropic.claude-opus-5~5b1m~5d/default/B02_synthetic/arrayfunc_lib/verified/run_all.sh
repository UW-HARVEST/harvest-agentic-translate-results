#!/usr/bin/env bash
# One-shot verification driver: builds both shared libraries, checks symbol
# parity, runs the whole differential suite in both profiles and across every
# feature combination, and mutation-tests the suite itself.
#
#   ./run_all.sh          full verification
#   ./run_all.sh quick    skip the mutation test

set -u
cd "$(dirname "$0")"

QUICK=${1:-}
STEP=0
FAILED=0

step() {
  STEP=$((STEP + 1))
  echo
  echo "############################################################"
  echo "# Step $STEP: $*"
  echo "############################################################"
}

fail() {
  echo "!!! FAILED: $*"
  FAILED=$((FAILED + 1))
}

# ---------------------------------------------------------------------------
step "Build the C shared library"
C_BUILD=../c_src/build
mkdir -p "$C_BUILD"
if (cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build .); then
  C_SO=$(readlink -f "$(ls "$C_BUILD"/*.so | head -1)")
  echo "C .so: $C_SO"
else
  fail "cmake build"
  exit 1
fi

# ---------------------------------------------------------------------------
step "cargo check (all targets, both profiles)"
for prof in "" "--release"; do
  if cargo check --offline --all-targets $prof 2>&1 | grep -qE '^(error|warning)'; then
    echo "  cargo check $prof produced diagnostics:"
    cargo check --offline --all-targets $prof 2>&1 | grep -E '^(error|warning)' | head -20
    if cargo check --offline --all-targets $prof 2>&1 | grep -qE '^error'; then
      fail "cargo check $prof"
    fi
  else
    echo "  cargo check $prof: clean"
  fi
done

# ---------------------------------------------------------------------------
step "Build the Rust cdylib (both profiles)"
cargo build --offline || fail "cargo build (debug)"
cargo build --offline --release || fail "cargo build (release)"
ls -l target/debug/libarrayfunc_lib.so target/release/libarrayfunc_lib.so

# ---------------------------------------------------------------------------
step "Symbol parity: nm -D on both .so files"
for prof in debug release; do
  RUST_SO="target/$prof/libarrayfunc_lib.so"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
    <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort))
  n_c=$(nm -D --defined-only "$C_SO" | wc -l)
  if [ -z "$missing" ]; then
    echo "  $prof: all $n_c C symbols exported by Rust; symbol diff is EMPTY"
  else
    fail "$prof: missing symbols: $(echo "$missing" | tr '\n' ' ')"
  fi
done

# ---------------------------------------------------------------------------
step "Differential suite (Phases B, C, D) - debug profile"
timeout 600 cargo test --offline || fail "cargo test (debug)"

# ---------------------------------------------------------------------------
step "Differential suite (Phases B, C, D) - release profile"
timeout 600 cargo test --offline --release || fail "cargo test (release)"

# ---------------------------------------------------------------------------
step "Feature matrix (every combination x both profiles)"
./check_features.sh || fail "check_features.sh"

# ---------------------------------------------------------------------------
if [ "$QUICK" != "quick" ]; then
  step "Mutation test: prove the suite would catch regressions"
  ./mutation_check.sh || fail "mutation_check.sh"
else
  echo
  echo "(skipping mutation test: 'quick' requested)"
fi

# ---------------------------------------------------------------------------
echo
echo "############################################################"
if [ "$FAILED" -eq 0 ]; then
  echo "# ALL VERIFICATION STEPS PASSED"
else
  echo "# $FAILED VERIFICATION STEP(S) FAILED"
fi
echo "############################################################"
exit $((FAILED > 0))
