#!/usr/bin/env bash
# Phase D driver: builds both libraries and runs the whole differential suite
# under every feature combination and every Rust build profile.
#
#   ./run_all.sh
#
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="--offline"       # the sandbox has no crates.io access
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C shared library
# --------------------------------------------------------------------------
step "building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -n1)"
echo "C   .so: $C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations declared in Cargo.toml
# --------------------------------------------------------------------------
FEATURES="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default") print a[1]}
' "$HERE/Cargo.toml")"

# Combination list: always the default build and the no-default-features build;
# plus each declared feature on its own and all of them together.
COMBOS=("<default>" "<none>")
if [ -n "$FEATURES" ]; then
  while read -r f; do [ -n "$f" ] && COMBOS+=("$f"); done <<< "$FEATURES"
  COMBOS+=("$(echo "$FEATURES" | tr '\n' ',' | sed 's/,$//')")
fi
echo "feature combinations to verify: ${COMBOS[*]}"

# --------------------------------------------------------------------------
# 3. Build + test each combination, against both Rust build profiles
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") fflags=() ;;
    "<none>")    fflags=(--no-default-features) ;;
    *)           fflags=(--no-default-features --features "$combo") ;;
  esac

  step "cargo check [$combo]"
  cargo check $CARGO_FLAGS "${fflags[@]}" || { echo "CHECK FAILED [$combo]"; FAILED=1; continue; }

  for profile in release debug; do
    step "building the Rust cdylib [$combo / $profile]"
    if [ "$profile" = release ]; then
      cargo build $CARGO_FLAGS --release "${fflags[@]}" || { FAILED=1; continue; }
    else
      cargo build $CARGO_FLAGS "${fflags[@]}" || { FAILED=1; continue; }
    fi
    RUST_SO="$HERE/target/$profile/libenvy_lib.so"
    [ -f "$RUST_SO" ] || { echo "missing $RUST_SO"; FAILED=1; continue; }

    step "symbol diff [$combo / $profile]"
    nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
    nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort > "${TMPDIR:-/tmp}/r_syms.txt"
    missing="$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt")"
    if [ -n "$missing" ]; then
      echo "SYMBOLS MISSING FROM THE RUST .so:"; echo "$missing"; FAILED=1
    else
      echo "symbol diff empty ($(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") symbols)"
    fi

    step "differential tests [$combo / $profile]"
    C_SO="$C_SO" RUST_SO="$RUST_SO" \
      timeout 600 cargo test $CARGO_FLAGS "${fflags[@]}" -- --test-threads=1 \
      || { echo "TESTS FAILED [$combo / $profile]"; FAILED=1; }
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES were reported above"
fi
exit "$FAILED"
