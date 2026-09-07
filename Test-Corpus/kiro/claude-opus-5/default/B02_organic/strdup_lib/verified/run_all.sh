#!/usr/bin/env bash
# Full verification gate: builds the C .so, then runs the differential test
# suite against every Rust build configuration and every feature combination.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here"
fails=0

step() { printf '\n=== %s ===\n' "$*"; }
check() {
  if [ "$1" -ne 0 ]; then printf '!! FAILED: %s\n' "$2"; fails=$((fails + 1));
  else printf '   ok: %s\n' "$2"; fi
}

# ---------------------------------------------------------------- C shared lib
step "Building the C shared library"
(cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
check $? "C libdriver.so built"
C_SO="$here/../c_src/build/libdriver.so"
test -f "$C_SO"; check $? "C .so exists at $C_SO"

# --------------------------------------------------- feature-combination sweep
# Derive the feature list from Cargo.toml instead of hardcoding it.
step "Enumerating feature combinations from Cargo.toml"
feats="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml \
        | grep -v '^default$' | tr -d ' ')"
if [ -z "$feats" ]; then
  echo "   Cargo.toml declares no [features]; the only configuration is the default."
  combos=("__default__" "__none__" "__all__")
else
  echo "   features: $feats"
  combos=("__default__" "__none__" "__all__")
  for f in $feats; do combos+=("$f"); done
fi

run_combo() {           # $1 = combo label
  local label="$1" flags=()
  case "$label" in
    __default__) flags=() ;;
    __none__)    flags=(--no-default-features) ;;
    __all__)     flags=(--all-features) ;;
    *)           flags=(--no-default-features --features "$label") ;;
  esac

  step "cargo check  [${label}]"
  timeout 600 cargo check "${flags[@]}" >/dev/null 2>&1
  check $? "cargo check [${label}]"

  # Debug .so (what `cargo test` builds) ...
  step "cargo test  [${label}]  against target/debug/libdriver.so"
  timeout 600 cargo build "${flags[@]}" >/dev/null 2>&1
  DIFF_C_SO="$C_SO" DIFF_RUST_SO="$here/target/debug/libdriver.so" \
    timeout 600 cargo test "${flags[@]}" 2>&1 | tail -n 20
  check "${PIPESTATUS[0]}" "tests [${label}] vs debug .so"

  # ... and the release .so, the artifact a real consumer links against.
  step "cargo test  [${label}]  against target/release/libdriver.so"
  timeout 600 cargo build --release "${flags[@]}" >/dev/null 2>&1
  DIFF_C_SO="$C_SO" DIFF_RUST_SO="$here/target/release/libdriver.so" \
    timeout 600 cargo test "${flags[@]}" 2>&1 | tail -n 20
  check "${PIPESTATUS[0]}" "tests [${label}] vs release .so"

  # Symbol parity for this configuration.
  step "nm -D symbol diff  [${label}]"
  for prof in debug release; do
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$here/target/$prof/libdriver.so" | awk '{print $NF}' | sort) \
      | grep '^<' && { echo "!! C symbols missing from Rust ($prof)"; fails=$((fails+1)); } \
      || echo "   ok: no C symbol missing from Rust .so ($prof)"
  done
}

for combo in "${combos[@]}"; do run_combo "$combo"; done

# ------------------------------------------------------------ binary/driver gate
step "Binary (driver executable) comparison"
if grep -q add_executable ../c_src/CMakeLists.txt || test -f src/main.rs; then
  echo "!! a binary target exists — stdout comparison must be implemented"
  fails=$((fails + 1))
else
  echo "   n/a: no add_executable in CMakeLists.txt and no src/main.rs — nothing to compare"
fi

step "SUMMARY"
if [ "$fails" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "$fails CHECK(S) FAILED"; fi
exit "$fails"
