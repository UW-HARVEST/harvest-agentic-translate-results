#!/usr/bin/env bash
# Full verification sweep: enumerates every valid Cargo feature combination,
# type-checks each, builds the C reference library, runs the differential tests
# for each combination, and diffs exported dynamic symbols between the two .so
# files. Run from the repository root.
set -uo pipefail
export RUST_TEST_THREADS=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_DIR="$ROOT/c_src"
R_DIR="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
step "Feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$R_DIR/Cargo.toml"
)

COMBOS=("")   # the empty combination == --no-default-features
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
for c in "${COMBOS[@]}"; do echo "  combo: '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. cargo check for every combination.
# ---------------------------------------------------------------------------
step "cargo check (all combinations)"
for c in "${COMBOS[@]}"; do
  if [[ -n "$c" ]]; then args=(--no-default-features --features "$c"); else args=(--no-default-features); fi
  if (cd "$R_DIR" && timeout 600 cargo check "${args[@]}") >/tmp/check.log 2>&1; then
    echo "  OK    '${c:-<none>}'"
  else
    echo "  FAIL  '${c:-<none>}'"; tail -30 /tmp/check.log; FAIL=1
  fi
done
# Also the default feature set, in case it differs from the empty combination.
if (cd "$R_DIR" && timeout 600 cargo check) >/tmp/check.log 2>&1; then
  echo "  OK    <default>"
else
  echo "  FAIL  <default>"; tail -30 /tmp/check.log; FAIL=1
fi

# ---------------------------------------------------------------------------
# 3. Build the C reference shared library.
# ---------------------------------------------------------------------------
step "Build C shared library"
mkdir -p "$C_DIR/build"
if (cd "$C_DIR/build" && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && timeout 600 cmake --build . >>/tmp/cmake.log 2>&1); then
  echo "  OK $(ls "$C_DIR"/build/*.so)"
else
  echo "  FAIL"; tail -30 /tmp/cmake.log; FAIL=1
fi

# ---------------------------------------------------------------------------
# 4. Differential tests + symbol comparison for every combination.
# ---------------------------------------------------------------------------
for c in "${COMBOS[@]}"; do
  step "cargo test '${c:-<none>}'"
  if [[ -n "$c" ]]; then args=(--no-default-features --features "$c"); else args=(--no-default-features); fi
  # Drop the stale cdylib so the harness rebuilds it for this combination.
  rm -f "$R_DIR"/target/debug/libdriver.so
  if (cd "$R_DIR" && timeout 600 cargo test "${args[@]}") >/tmp/test.log 2>&1; then
    grep -E '^test result:' /tmp/test.log | sed 's/^/  /'
  else
    echo "  FAIL"; tail -40 /tmp/test.log; FAIL=1
  fi

  step "Exported symbol diff '${c:-<none>}'"
  if [[ -n "$c" ]]; then bargs=(--no-default-features --features "$c"); else bargs=(--no-default-features); fi
  (cd "$R_DIR" && timeout 600 cargo build --release "${bargs[@]}") >/tmp/build.log 2>&1 \
    || { echo "  release build FAIL"; tail -20 /tmp/build.log; FAIL=1; }
  syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u; }
  syms "$C_DIR/build/libdriver.so" >/tmp/c.syms
  syms "$R_DIR/target/release/libdriver.so" >/tmp/r.syms
  # Every symbol the C library exports must also be exported by the Rust library.
  missing=$(comm -23 /tmp/c.syms /tmp/r.syms)
  if [[ -n "$missing" ]]; then
    echo "  MISSING from Rust .so:"; echo "$missing" | sed 's/^/    /'; FAIL=1
  else
    echo "  OK: all $(wc -l </tmp/c.syms) C-exported symbols present in the Rust .so"
    sed 's/^/    C exports: /' /tmp/c.syms
  fi
done

step "RESULT"
if ((FAIL)); then echo "FAILURES PRESENT"; exit 1; else echo "ALL CHECKS PASSED"; fi
