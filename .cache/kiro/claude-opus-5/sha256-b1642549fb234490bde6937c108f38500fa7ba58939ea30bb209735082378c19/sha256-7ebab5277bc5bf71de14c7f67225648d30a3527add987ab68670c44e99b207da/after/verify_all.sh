#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration, then diff the exported dynamic symbols.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST="$ROOT/translation"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; FAIL=1; }

step "Build C ground truth"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "C shared library" || bad "C shared library"

# translation/Cargo.toml declares no [features], so the complete set of valid
# feature combinations is the single empty one. Both spellings are exercised.
COMBOS=("--no-default-features" "")

step "cargo check, every feature combination"
for combo in "${COMBOS[@]}"; do
  label="cargo check ${combo:-<default>}"
  # shellcheck disable=SC2086
  if ( cd "$RUST" && timeout 600 cargo check $combo >/dev/null 2>&1 ); then
    ok "$label"
  else
    bad "$label"
  fi
done

step "cargo test, every feature combination (dev cdylib)"
for combo in "${COMBOS[@]}"; do
  label="cargo test ${combo:-<default>}"
  # `cargo test` does not emit the cdylib itself, so build it explicitly and
  # point the harness at it rather than letting it fall back to another profile.
  # shellcheck disable=SC2086
  if ( cd "$RUST" && timeout 600 cargo build $combo >/dev/null 2>&1 ) \
     && ( cd "$RUST" \
          && DRIVER_RUST_SO="$RUST/target/debug/libdriver.so" \
             timeout 600 cargo test $combo >/dev/null 2>&1 ); then
    ok "$label"
  else
    bad "$label"
  fi
done

step "cargo test against the release cdylib (panic = abort)"
for combo in "${COMBOS[@]}"; do
  label="release artifact ${combo:-<default>}"
  # shellcheck disable=SC2086
  if ( cd "$RUST" && timeout 600 cargo build --release $combo >/dev/null 2>&1 ) \
     && ( cd "$RUST" \
          && DRIVER_RUST_SO="$RUST/target/release/libdriver.so" \
             timeout 600 cargo test $combo >/dev/null 2>&1 ); then
    ok "$label"
  else
    bad "$label"
  fi
done

step "Exported dynamic symbols: C vs Rust"
syms() { nm -D --defined-only "$1" 2>/dev/null | awk '$2 ~ /^[TtWwDdBb]$/ {print $3}' | sort -u; }
for prof in debug release; do
  RS="$RUST/target/$prof/libdriver.so"
  [ -f "$RS" ] || continue
  missing="$(comm -23 <(syms "$C_SO") <(syms "$RS"))"
  if [ -z "$missing" ]; then
    ok "$prof cdylib exports every C symbol"
  else
    bad "$prof cdylib is missing: $(echo "$missing" | tr '\n' ' ')"
  fi
done
printf 'C   exports: %s\n' "$(syms "$C_SO" | tr '\n' ' ')"
printf 'Rust exports: %s\n' "$(syms "$RUST/target/release/libdriver.so" | tr '\n' ' ')"

step "Result"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS VERIFIED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
