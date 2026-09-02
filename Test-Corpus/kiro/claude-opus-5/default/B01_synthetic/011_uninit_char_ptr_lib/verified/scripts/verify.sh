#!/usr/bin/env bash
# Full verification pipeline: build the C reference, build the Rust cdylib,
# check symbol parity, and run the differential suite under every feature
# combination declared in Cargo.toml.
#
# Usage:  translation/scripts/verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$CRATE/.." && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
TIMEOUT=${TIMEOUT:-600}

rc=0
step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; rc=1; }

step "Build the C reference shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout "$TIMEOUT" cmake --build . >/dev/null ) \
  || fail "C build"
[ -f "$C_SO" ] || fail "missing $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
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
echo "declared non-default features: ${FEATURES[*]:-<none>}"

# Combination list: always the default set and the empty set; then every
# declared feature on its own, then all of them together. With no [features]
# table this reduces to the two configurations that actually exist.
COMBOS=("default:" "none:--no-default-features")
for f in "${FEATURES[@]:-}"; do
  [ -n "$f" ] || continue
  COMBOS+=("$f:--no-default-features --features $f")
done
if [ "${#FEATURES[@]}" -gt 1 ]; then
  all=$(IFS=, ; echo "${FEATURES[*]}")
  COMBOS+=("all:--no-default-features --features $all")
fi

# ---------------------------------------------------------------------------
# Per-combination: cargo check, build the cdylib, symbol diff, run the suite.
# ---------------------------------------------------------------------------
for entry in "${COMBOS[@]}"; do
  name=${entry%%:*}
  flags=${entry#*:}
  step "combination: $name  (cargo flags: ${flags:-<none>})"

  ( cd "$CRATE" && timeout "$TIMEOUT" cargo check --release $flags 2>&1 | tail -3 ) \
    || fail "$name: cargo check"

  ( cd "$CRATE" && timeout "$TIMEOUT" cargo build --release $flags 2>&1 | tail -3 ) \
    || fail "$name: cargo build --release"

  R_SO="$CRATE/target/release/libdriver.so"
  if [ ! -f "$R_SO" ]; then
    fail "$name: missing $R_SO"
    continue
  fi

  # Symbol parity: every symbol the C .so defines must be defined by the Rust
  # .so under the same name.
  missing=$(comm -23 \
    <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort -u) \
    <(nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort -u))
  if [ -n "$missing" ]; then
    fail "$name: Rust .so is missing C symbols: $(echo "$missing" | tr '\n' ' ')"
  else
    echo "symbol parity: OK ($(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort -u | tr '\n' ' '))"
  fi

  # Unresolvable imports: RTLD_NOW binds everything at load time.
  if ! python3 -c "
import ctypes, sys
ctypes.CDLL('$R_SO', mode=ctypes.RTLD_LOCAL)
" 2>/dev/null; then
    fail "$name: dlopen(RTLD_NOW) of the Rust .so failed (unresolved imports)"
  else
    echo "import resolution: OK"
  fi

  ( cd "$CRATE" && timeout "$TIMEOUT" cargo test --release $flags --test differential \
      -- --test-threads=1 2>&1 | tail -4 ) \
    || fail "$name: differential suite (release cdylib)"

  # The cdylib's stack geometry must not depend on the cargo profile, so run the
  # same suite against the dev-profile object as well.
  ( cd "$CRATE" && timeout "$TIMEOUT" cargo build $flags 2>&1 | tail -2 ) \
    || fail "$name: cargo build (dev)"
  D_SO="$CRATE/target/debug/libdriver.so"
  if [ -f "$D_SO" ]; then
    ( cd "$CRATE" && DRIVER_RUST_SO="$D_SO" timeout "$TIMEOUT" \
        cargo test --release $flags --test differential -- --test-threads=1 2>&1 | tail -4 ) \
      || fail "$name: differential suite (dev cdylib)"
  else
    fail "$name: missing $D_SO"
  fi
done

step "Portable (non-x86-64) fallback path compiles"
# The naked-asm `driver` is x86-64 only; make sure the portable fallback still
# type-checks by building for a non-x86-64 target if its std is installed.
if rustup target list --installed 2>/dev/null | grep -q aarch64-unknown-linux-gnu; then
  ( cd "$CRATE" && timeout "$TIMEOUT" cargo check --release --target aarch64-unknown-linux-gnu 2>&1 | tail -3 ) \
    || fail "aarch64 cargo check"
else
  echo "skipped: aarch64-unknown-linux-gnu std not installed"
fi

step "Summary"
if [ "$rc" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$rc"
