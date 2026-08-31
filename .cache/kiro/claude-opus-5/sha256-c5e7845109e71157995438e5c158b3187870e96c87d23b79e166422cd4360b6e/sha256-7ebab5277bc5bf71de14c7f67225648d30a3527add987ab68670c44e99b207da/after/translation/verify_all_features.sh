#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and, for each one:
#   1. cargo check --no-default-features --features <combo>
#   2. cargo build --release --no-default-features --features <combo>
#   3. compares exported dynamic symbols against the C .so
#   4. cargo test --no-default-features --features <combo>
#
# Also runs the crate's default configuration. The C library is built once,
# since c_src/CMakeLists.txt exposes no build-time options.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
c_build="$root/c_src/build"
TIMEOUT=${TIMEOUT:-600}
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C library ---
step "Building C shared library"
mkdir -p "$c_build"
( cd "$c_build" \
  && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && timeout "$TIMEOUT" cmake --build . ) > /tmp/c_build.log 2>&1 \
  || { echo "C build FAILED"; tail -20 /tmp/c_build.log; exit 1; }
c_so="$c_build/libdriver.so"
echo "built $c_so"

# ------------------------------------------------- feature enumeration --------
# Names under [features] in Cargo.toml, excluding "default".
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
      if ($0 != "default" && $0 != "") print
    }
  ' "$here/Cargo.toml"
)

n=${#features[@]}
echo "declared features (${n}): ${features[*]:-<none>}"

combos=()
for (( mask = 0; mask < (1 << n); mask++ )); do
  sel=()
  for (( b = 0; b < n; b++ )); do
    (( mask & (1 << b) )) && sel+=("${features[b]}")
  done
  combos+=("$(IFS=,; echo "${sel[*]}")")
done

echo "feature combinations to verify: ${#combos[@]} (plus the default config)"

# --------------------------------------------------------------- verify -------
verify() {           # verify <label> <cargo flags...>
  local label="$1"; shift
  step "$label"

  if ! timeout "$TIMEOUT" cargo check "$@" > /tmp/check.log 2>&1; then
    echo "cargo check FAILED"; tail -30 /tmp/check.log; fail=1; return
  fi
  echo "check ok"

  if ! timeout "$TIMEOUT" cargo build --release "$@" > /tmp/build.log 2>&1; then
    echo "cargo build --release FAILED"; tail -30 /tmp/build.log; fail=1; return
  fi
  echo "build ok"

  local rust_so="$here/target/release/libdriver.so"
  local missing
  missing="$(comm -23 \
    <(nm -D --defined-only "$c_so"    | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))"
  if [[ -n "$missing" ]]; then
    echo "SYMBOL MISMATCH — exported by C but not by Rust:"; echo "$missing"; fail=1
  else
    echo "symbols ok ($(nm -D --defined-only "$c_so" | awk '{print $3}' | tr '\n' ' '))"
  fi

  if ! RUST_DRIVER_SO="$rust_so" C_DRIVER_SO="$c_so" \
       timeout "$TIMEOUT" cargo test "$@" > /tmp/test.log 2>&1; then
    echo "cargo test FAILED"; grep -E "panicked|mismatch|test result" /tmp/test.log | head -30; fail=1; return
  fi
  grep -E "^test result" /tmp/test.log
}

cd "$here"
verify "default features" 
for combo in "${combos[@]}"; do
  verify "--no-default-features --features '${combo}'" \
    --no-default-features --features "$combo"
done

step "SUMMARY"
if (( fail )); then echo "FAILURES PRESENT"; exit 1; fi
echo "all feature combinations: check + build + symbol parity + differential tests PASSED"
