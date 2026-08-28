#!/usr/bin/env bash
# Builds the C reference .so and the Rust cdylib, then runs the differential
# tests for every valid feature combination.
#
# Cargo.toml declares no [features], so the only valid configuration is the
# default one; the loop below is derived from Cargo.toml so it stays correct if
# features are ever added.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR=/tmp/doubleneg-verify
mkdir -p "$LOGDIR"

echo "=== Building C reference shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .
) >"$LOGDIR/c-build.log" 2>&1 || {
  echo "C build FAILED"; tail -30 "$LOGDIR/c-build.log"; exit 1
}
ls "$ROOT"/c_src/build/lib*.so

# --- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("<default>")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("${combo:-<none>}")
  done
fi

echo "=== Feature combinations to verify: ${COMBOS[*]} ==="

STATUS=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") ARGS=() ;;
    "<none>")    ARGS=(--no-default-features) ;;
    *)           ARGS=(--no-default-features --features "$combo") ;;
  esac
  slug="$(echo "$combo" | tr -c 'A-Za-z0-9_-' '_' | tr -d '\n')"

  echo
  echo "--- [$combo] cargo check --all-targets ---"
  if ! (cd "$CRATE" && timeout 600 cargo check --all-targets "${ARGS[@]}") \
      >"$LOGDIR/check-$slug.log" 2>&1; then
    echo "CHECK FAILED"; tail -40 "$LOGDIR/check-$slug.log"; STATUS=1; continue
  fi
  echo "check ok"

  echo "--- [$combo] cargo build (cdylib) ---"
  if ! (cd "$CRATE" && timeout 600 cargo build "${ARGS[@]}") \
      >"$LOGDIR/build-$slug.log" 2>&1; then
    echo "BUILD FAILED"; tail -40 "$LOGDIR/build-$slug.log"; STATUS=1; continue
  fi
  ls -la "$CRATE/target/debug/libdoubleneg_lib.so"

  echo "--- [$combo] cargo test ---"
  if ! (cd "$CRATE" && timeout 600 cargo test "${ARGS[@]}" -- --test-threads=1) \
      >"$LOGDIR/test-$slug.log" 2>&1; then
    echo "TESTS FAILED"; tail -60 "$LOGDIR/test-$slug.log"; STATUS=1; continue
  fi
  grep -E '^test result:' "$LOGDIR/test-$slug.log"

  echo "--- [$combo] nm -D symbol parity ---"
  diff <(nm -D --defined-only "$ROOT"/c_src/build/lib*.so | awk '{print $3}' | grep -v '^_' | sort) \
       <(nm -D --defined-only "$CRATE/target/debug/libdoubleneg_lib.so" | awk '{print $3}' | grep -v '^_' | sort) \
       >"$LOGDIR/nm-$slug.diff"
  echo "C-only symbols (must be empty):"
  grep '^<' "$LOGDIR/nm-$slug.diff" || echo "  (none)"

  # The shipped artifact is the optimised cdylib (release enables panic=abort
  # and lets LLVM constant-fold the libm calls), so verify it too.
  echo "--- [$combo] release cdylib ---"
  if ! (cd "$CRATE" && timeout 600 cargo build --release "${ARGS[@]}") \
      >"$LOGDIR/relbuild-$slug.log" 2>&1; then
    echo "RELEASE BUILD FAILED"; tail -40 "$LOGDIR/relbuild-$slug.log"; STATUS=1; continue
  fi
  if ! (cd "$CRATE" && RUST_SO_PATH="$CRATE/target/release/libdoubleneg_lib.so" \
        timeout 600 cargo test "${ARGS[@]}" -- --test-threads=1) \
      >"$LOGDIR/reltest-$slug.log" 2>&1; then
    echo "RELEASE TESTS FAILED"; tail -60 "$LOGDIR/reltest-$slug.log"; STATUS=1; continue
  fi
  grep -E '^test result:' "$LOGDIR/reltest-$slug.log"
  diff <(nm -D --defined-only "$ROOT"/c_src/build/lib*.so | awk '{print $3}' | grep -v '^_' | sort) \
       <(nm -D --defined-only "$CRATE/target/release/libdoubleneg_lib.so" | awk '{print $3}' | grep -v '^_' | sort) \
       >"$LOGDIR/nm-release-$slug.diff"
  echo "C-only symbols in release (must be empty):"
  grep '^<' "$LOGDIR/nm-release-$slug.diff" || echo "  (none)"
done

echo
if [ "$STATUS" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$STATUS"
