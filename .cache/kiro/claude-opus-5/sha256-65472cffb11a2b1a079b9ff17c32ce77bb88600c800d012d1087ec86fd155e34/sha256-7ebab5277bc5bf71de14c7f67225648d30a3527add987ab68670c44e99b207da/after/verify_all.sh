#!/usr/bin/env bash
# Verifies the Rust translation against the C reference for every build-time
# configuration: each Cargo feature combination, in both dev and release
# profiles.
#
# Usage: ./verify_all.sh          (from the repository root)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_SRC="$ROOT/c_src"
RUST="$ROOT/translation"
TIMEOUT=600
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in translation/Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[[:space:]"]/, "", a[1]);
                      if (a[1] != "" && a[1] != "default") print a[1] }
  ' "$RUST/Cargo.toml"
)

# Powerset of the declared features; "" denotes "no features at all".
COMBOS=("")
for f in "${FEATURES[@]}"; do
  existing=("${COMBOS[@]}")
  for c in "${existing[@]}"; do
    if [ -z "$c" ]; then COMBOS+=("$f"); else COMBOS+=("$c,$f"); fi
  done
done

step "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then echo "  <none>  (--no-default-features)"; else echo "  $c"; fi
done
if [ ${#FEATURES[@]} -eq 0 ]; then
  echo "  (translation/Cargo.toml declares no [features]; c_src/CMakeLists.txt"
  echo "   declares no build options either, so there is one configuration.)"
fi

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Building the C reference .so"
mkdir -p "$C_SRC/build"
(
  cd "$C_SRC/build" &&
    timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    timeout $TIMEOUT cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$C_SRC/build" -name 'libdriver.so' | head -1)"
echo "  $C_SO"

# ---------------------------------------------------------------------------
# 3. cargo check every combination
# ---------------------------------------------------------------------------
for c in "${COMBOS[@]}"; do
  label="${c:-<none>}"
  step "cargo check --no-default-features --features '$c'  [$label]"
  ( cd "$RUST" && timeout $TIMEOUT cargo check --no-default-features --features "$c" 2>&1 | tail -5 ) \
    || { echo "CHECK FAILED: $label"; FAILED=1; }
done

# ---------------------------------------------------------------------------
# 4. Symbol parity + differential tests, per combination and per profile
# ---------------------------------------------------------------------------
for c in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features='${c:-<none>}' profile=$profile"
    relflag=(); [ "$profile" = release ] && relflag=(--release)

    step "Building Rust .so  [$label]"
    ( cd "$RUST" && timeout $TIMEOUT cargo build "${relflag[@]}" \
        --no-default-features --features "$c" 2>&1 | tail -5 ) \
      || { echo "BUILD FAILED: $label"; FAILED=1; continue; }

    RUST_SO="$RUST/target/$([ "$profile" = release ] && echo release || echo debug)/libdriver.so"

    step "Symbol comparison (nm -D)  [$label]"
    c_syms="$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)"
    r_syms="$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)"
    echo "  C   : $(echo "$c_syms" | tr '\n' ' ')"
    echo "  Rust: $(echo "$r_syms" | tr '\n' ' ')"
    missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
    if [ -n "$missing" ]; then
      echo "  MISSING FROM RUST .so: $(echo "$missing" | tr '\n' ' ')"
      FAILED=1
    else
      echo "  OK: every C export is present in the Rust .so"
    fi

    step "cargo test  [$label]"
    ( cd "$RUST" && timeout $TIMEOUT cargo test "${relflag[@]}" \
        --no-default-features --features "$c" 2>&1 | tail -25 ) \
      || { echo "TESTS FAILED: $label"; FAILED=1; }
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (see above)"
fi
exit "$FAILED"
