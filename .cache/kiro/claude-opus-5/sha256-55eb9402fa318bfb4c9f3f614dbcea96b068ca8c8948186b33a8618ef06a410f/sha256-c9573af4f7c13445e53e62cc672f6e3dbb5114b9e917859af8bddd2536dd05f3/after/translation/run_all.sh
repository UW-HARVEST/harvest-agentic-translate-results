#!/usr/bin/env bash
# Full verification sweep: builds the C .so and the Rust cdylib, then runs the
# differential suite under every feature combination and both profiles.
#
# Usage:  ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

echo "=== building the C shared library ==============================="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (there is no [features]
# section in this crate, so this reduces to the default + --no-default-features
# runs, but it is derived, not assumed).
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' "$CRATE/Cargo.toml" | grep -v '^default$' | tr '\n' ' ')
echo "declared features: [${FEATURES:-<none>}]"

COMBOS=("default")
COMBOS+=("--no-default-features")
if [ -n "${FEATURES// /}" ]; then
  COMBOS+=("--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
  # pairwise combinations
  for a in $FEATURES; do
    for b in $FEATURES; do
      [ "$a" \< "$b" ] && COMBOS+=("--no-default-features --features $a,$b")
    done
  done
fi

for profile in debug release; do
  PROFILE_FLAG=""
  [ "$profile" = release ] && PROFILE_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    flags=""
    [ "$combo" != "default" ] && flags="$combo"
    echo
    echo "=== profile=$profile features=[${flags:-default}] ==============="
    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo build $PROFILE_FLAG $flags ) >/dev/null 2>&1 \
      || { echo "  BUILD FAILED"; FAIL=1; continue; }
    RUST_SO="$CRATE/target/$profile/libupdate_md5_lib.so"
    [ -f "$RUST_SO" ] || { echo "  missing $RUST_SO"; FAIL=1; continue; }

    echo "  symbol diff (C -> Rust):"
    diff <(nm -D --defined-only --format=posix "$C_SO" | awk '$2=="T"{print $1}' | sort) \
         <(nm -D --defined-only --format=posix "$RUST_SO" | awk '$2=="T"{print $1}' | sort) \
         | sed 's/^/    /' || true
    missing=$(comm -23 \
      <(nm -D --defined-only --format=posix "$C_SO" | awk '$2=="T"{print $1}' | sort) \
      <(nm -D --defined-only --format=posix "$RUST_SO" | awk '$2=="T"{print $1}' | sort) | wc -l)
    echo "  symbols exported by C but missing from Rust: $missing"
    [ "$missing" -eq 0 ] || FAIL=1

    # shellcheck disable=SC2086
    ( cd "$CRATE" && C_SO="$C_SO" RUST_SO="$RUST_SO" \
        timeout 600 cargo test $PROFILE_FLAG $flags 2>&1 | tail -5 ) || FAIL=1
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
