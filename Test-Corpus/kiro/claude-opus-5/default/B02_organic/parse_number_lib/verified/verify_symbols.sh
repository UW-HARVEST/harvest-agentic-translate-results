#!/usr/bin/env bash
# Phase D — symbol parity + feature-combination gate.
# Run from the `translation/` directory. Exits non-zero on any gate failure.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
RUST_SO="$ROOT/translation/target/release/libdriver.so"
fail=0

hdr() { printf '\n=== %s ===\n' "$1"; }

hdr "build C .so"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FAIL: C build"; exit 1; }

hdr "enumerate feature combinations"
# Mechanically extract [features] keys from Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
  }' Cargo.toml)
if [ -n "$FEATURES" ]; then
  echo "declared features: $FEATURES"
else
  echo "no [features] table in Cargo.toml -> single (default) build configuration"
fi

# Build the list of configurations to verify.
CONFIGS=("" "--no-default-features")
if [ -n "$FEATURES" ]; then
  # Power set of the declared features (guard against combinatorial blow-up).
  n=$(printf '%s\n' $FEATURES | wc -l)
  if [ "$n" -le 12 ]; then
    mapfile -t FARR <<<"$FEATURES"
    total=$((1 << n))
    for ((m = 0; m < total; m++)); do
      combo=""
      for ((b = 0; b < n; b++)); do
        if (((m >> b) & 1)); then combo="$combo,${FARR[b]}"; fi
      done
      CONFIGS+=("--no-default-features --features ${combo#,}")
    done
  else
    echo "WARN: $n features -> power set too large; verifying singletons only"
    for f in $FEATURES; do
      CONFIGS+=("--no-default-features --features $f")
    done
  fi
fi

for cfg in "${CONFIGS[@]}"; do
  label="${cfg:-<default>}"

  hdr "cargo check   [$label]"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --all-targets $cfg 2>&1 | tail -3; then
    echo "FAIL: cargo check [$label]"; fail=1; continue
  fi

  hdr "cargo build --release   [$label]"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $cfg >/dev/null 2>&1; then
    echo "FAIL: release build [$label]"; fail=1; continue
  fi

  hdr "symbol parity   [$label]"
  c_syms=$(nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
  echo "C exports   : $(printf '%s\n' "$c_syms" | grep -c .)"
  echo "Rust exports: $(printf '%s\n' "$r_syms" | grep -c .)"
  if [ -n "$missing" ]; then
    echo "FAIL: symbols in C .so missing from Rust .so [$label]:"
    printf '  %s\n' $missing
    fail=1
  else
    echo "OK: 0 missing symbols"
  fi

  # No unresolved non-libc symbols.
  if ldd -r "$RUST_SO" 2>&1 | grep -qi 'undefined symbol'; then
    echo "FAIL: unresolved symbols in Rust .so [$label]"
    ldd -r "$RUST_SO" 2>&1 | grep -i 'undefined symbol'
    fail=1
  else
    echo "OK: no unresolved symbols (ldd -r)"
  fi

  hdr "differential tests   [$label]"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $cfg 2>&1 | grep -E 'test result|panicked|FAILED'; then
    echo "FAIL: tests did not run [$label]"; fail=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $cfg >/dev/null 2>&1; then
    echo "FAIL: differential tests [$label]"; fail=1
  fi
done

hdr "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASE D GATES PASSED"
else
  echo "GATE FAILURES PRESENT"
fi
exit "$fail"
