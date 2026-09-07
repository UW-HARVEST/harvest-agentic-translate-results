#!/usr/bin/env bash
# Full differential verification of the Rust translation against the C original.
#
# Phase A artifacts: SYMBOLS.md, ERRORS.md, CONFIGS.md
# Phase B: tests/phase_b_configs.rs   (one case per CONFIGS.md row)
# Phase C: tests/phase_c_errors.rs    (one case per ERRORS.md row)
# Phase D: tests/phase_d_symbols.rs   + the feature/profile sweep below
#
# Usage: ./verify.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
CARGO_FLAGS="--offline"
FAILED=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "libString_Slice.so (C)" || bad "C build"
C_SO="$ROOT/c_src/build/libString_Slice.so"

# ---------------------------------------------------------------------------
step "Enumerating feature combinations"
# ---------------------------------------------------------------------------
# Mechanically extract the [features] table; if the crate declares none there is
# exactly one configuration and the three canonical invocations coincide.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { print $1 }
' "$CRATE_DIR/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "  crate declares no [features]; one configuration only"
  COMBOS=("default" "no-default")
else
  echo "  features: $FEATURES"
  COMBOS=("default" "no-default")
  for f in $FEATURES; do COMBOS+=("only:$f"); done
  COMBOS+=("all")
fi

flags_for() {
  case "$1" in
    default)    echo "" ;;
    no-default) echo "--no-default-features" ;;
    all)        echo "--all-features" ;;
    only:*)     echo "--no-default-features --features ${1#only:}" ;;
  esac
}

# ---------------------------------------------------------------------------
step "cargo check / clippy across every feature combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  ( cd "$CRATE_DIR" && cargo check $CARGO_FLAGS $(flags_for "$combo") --all-targets ) >/dev/null 2>&1 \
    && ok "cargo check [$combo]" || bad "cargo check [$combo]"
done

# ---------------------------------------------------------------------------
step "Phase B + C + D differential tests, per feature combination x profile"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    fl="$(flags_for "$combo")"
    rel_flag=""
    [ "$profile" = release ] && rel_flag="--release"

    # Build the cdylib for this combination/profile, then point the test suite at
    # exactly that artifact via STRING_SLICE_RUST_SO.
    # shellcheck disable=SC2046
    ( cd "$CRATE_DIR" && cargo build $CARGO_FLAGS $rel_flag $fl ) >/dev/null 2>&1 \
      || { bad "build cdylib [$combo/$profile]"; continue; }

    RUST_SO="$CRATE_DIR/target/$profile/libString_Slice.so"
    [ -f "$RUST_SO" ] || { bad "missing cdylib [$combo/$profile]"; continue; }

    # Hard symbol-parity gate, independent of the test binary.
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u))
    if [ -n "$missing" ]; then
      bad "symbol parity [$combo/$profile]: Rust .so missing: $(echo "$missing" | tr '\n' ' ')"
    else
      ok "symbol parity [$combo/$profile] (0 missing)"
    fi

    # shellcheck disable=SC2046
    ( cd "$CRATE_DIR" && STRING_SLICE_RUST_SO="$RUST_SO" \
        cargo test $CARGO_FLAGS $fl ) >${TMPDIR:-/tmp}/ss_test.$$ 2>&1
    if [ $? -eq 0 ]; then
      n=$(grep -c '\.\.\. ok$' ${TMPDIR:-/tmp}/ss_test.$$)
      ok "differential tests [$combo/$profile] ($n cases)"
    else
      bad "differential tests [$combo/$profile]"
      tail -40 ${TMPDIR:-/tmp}/ss_test.$$
    fi
    rm -f ${TMPDIR:-/tmp}/ss_test.$$
  done
done

# ---------------------------------------------------------------------------
step "Phase A artifacts present"
# ---------------------------------------------------------------------------
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  [ -s "$CRATE_DIR/$f" ] && ok "$f" || bad "$f missing or empty"
done

# ---------------------------------------------------------------------------
step "Binary executable check"
# ---------------------------------------------------------------------------
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "c_src builds an executable but no stdout comparison is wired up"
else
  ok "c_src builds no executable (library only) - stdout compared per-call instead"
fi

# ---------------------------------------------------------------------------
if [ "$FAILED" -eq 0 ]; then
  printf '\n\033[1;32mVERIFICATION COMPLETE: all gates passed.\033[0m\n\n'
else
  printf '\n\033[1;31mVERIFICATION FAILED.\033[0m\n\n'
fi
exit "$FAILED"
