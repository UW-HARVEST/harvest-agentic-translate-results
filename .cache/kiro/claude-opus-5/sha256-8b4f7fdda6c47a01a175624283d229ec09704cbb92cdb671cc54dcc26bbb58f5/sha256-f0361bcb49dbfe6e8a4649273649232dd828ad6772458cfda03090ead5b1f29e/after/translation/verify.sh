#!/usr/bin/env bash
# verify.sh — full C-vs-Rust differential verification (Phases A-D).
#
# Rebuilds both shared libraries, checks exported-symbol parity, then runs the
# whole differential suite against every Rust build profile and every Cargo
# feature combination.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_BUILD="$ROOT/c_src/build"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
note() { printf '  %s\n' "$*"; }
bad()  { printf '  FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | sort | head -1)"
note "C  .so: $C_SO"

# ---------------------------------------------------------------------------
step "Build the Rust cdylib (dev + release)"
# `cargo test` does NOT rebuild a cdylib-only artifact, so build explicitly.
timeout 600 cargo build          >/dev/null 2>&1 || { bad "cargo build (dev)"; exit 1; }
timeout 600 cargo build --release >/dev/null 2>&1 || { bad "cargo build (release)"; exit 1; }
note "dev     .so: target/debug/libmathop_lib.so"
note "release .so: target/release/libmathop_lib.so"

# ---------------------------------------------------------------------------
step "Phase A/D — exported-symbol parity (nm -D)"
for RS in target/debug/libmathop_lib.so target/release/libmathop_lib.so; do
  DIFF="$(comm -3 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
      <(nm -D --defined-only "$RS"   | awk '{print $3}' | sort))"
  if [ -n "$DIFF" ]; then
    bad "symbol diff vs $RS:"; printf '%s\n' "$DIFF"
  else
    note "$RS: 0 missing / 0 extra ($(nm -D --defined-only "$RS" | wc -l) symbols)"
  fi
done

step "Phase A — undefined non-libc symbols in the Rust .so"
UNDEF="$(nm -D --undefined-only target/release/libmathop_lib.so \
  | awk '{print $NF}' \
  | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__cxa_\|^__gmon_start__\|^gettid$\|^statx$')"
if [ -n "$UNDEF" ]; then bad "non-libc undefined symbols:"; printf '%s\n' "$UNDEF"
else note "none"; fi

# ---------------------------------------------------------------------------
step "Phase D — enumerate Cargo feature combinations"
FEATS="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,"");print}' Cargo.toml)"
if [ -z "$FEATS" ]; then
  note "Cargo.toml declares no [features] — the only configuration is the default one"
  COMBOS=("__default__")
else
  note "features: $FEATS"
  COMBOS=("__default__" "__none__")
  for f in $FEATS; do COMBOS+=("$f"); done
fi

step "Phase D — is there a binary driver to compare stdout for?"
if grep -q '^\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ] || ls src/bin/*.rs >/dev/null 2>&1; then
  bad "a Rust binary exists but no C executable comparison is wired up"
elif grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "c_src builds an executable but the Rust side has none"
else
  note "neither side builds an executable (c_src: add_library SHARED only) — N/A"
fi

# ---------------------------------------------------------------------------
step "Phases B+C — differential suite, every profile x every feature combo"
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) FLAGS=() ; label="default features" ;;
    __none__)    FLAGS=(--no-default-features) ; label="no default features" ;;
    *)           FLAGS=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac
  for prof in debug release; do
    SO="$PWD/target/$prof/libmathop_lib.so"
    printf '\n--- %s | Rust .so profile: %s ---\n' "$label" "$prof"
    if ! MATHOP_RUST_SO="$SO" timeout 600 cargo test "${FLAGS[@]}" 2>&1 \
         | grep -E 'test result|^error|FAILED'; then
      bad "$label / $prof: cargo test produced no result line"
    fi
    if MATHOP_RUST_SO="$SO" timeout 600 cargo test "${FLAGS[@]}" >/dev/null 2>&1; then
      note "PASS ($label, $prof)"
    else
      bad "$label / $prof"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Result"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "THERE WERE FAILURES"; fi
exit "$FAIL"
