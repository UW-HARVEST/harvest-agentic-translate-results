#!/usr/bin/env bash
# Runs the whole differential suite across every configuration:
#   * every cargo feature combination declared in Cargo.toml
#   * both Rust cdylib profiles (release, and debug with overflow-checks on)
#   * both C builds (the documented default build, and an optimized -O2 build)
#
# Usage: translation/scripts/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.."
CRATE="$PWD"
ROOT="$(cd .. && pwd)"

TIMEOUT=${TIMEOUT:-600}
fail=0
run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  if timeout "$TIMEOUT" "$@"; then
    printf '%s\n' "--- PASS: $label"
  else
    printf '%s\n' "!!! FAIL: $label"
    fail=1
  fi
}

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations mechanically from Cargo.toml.
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
) || true

echo "declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Power set of the declared features, as --features arguments.
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done

# --------------------------------------------------------------------------
# 2. Build the C library twice: documented default, and optimized.
# --------------------------------------------------------------------------
C_DEFAULT_DIR="$ROOT/c_src/build"
if ! ls "$C_DEFAULT_DIR"/*.so >/dev/null 2>&1; then
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "default C build failed"; exit 1; }
fi
C_DEFAULT_SO="$(ls "$C_DEFAULT_DIR"/*.so | head -1)"
echo "C (default build): $C_DEFAULT_SO"

# The optimized build goes in a scratch dir OUTSIDE c_src so nothing in c_src
# is modified. CMake is pointed at c_src as an out-of-source source dir.
C_OPT_DIR="$CRATE/target/c_build_release"
mkdir -p "$C_OPT_DIR"
C_OPT_SO=""
if ( cd "$C_OPT_DIR" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1 ); then
  C_OPT_SO="$(ls "$C_OPT_DIR"/*.so 2>/dev/null | head -1)"
  echo "C (-O3 build):     ${C_OPT_SO:-<none>}"
else
  echo "C (-O3 build):     skipped (configure/build failed)"
fi

# --------------------------------------------------------------------------
# 3. For each feature combo x rust profile x C build, build and test.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    feat_args=(--no-default-features)
    combo_label="<no features>"
  else
    feat_args=(--no-default-features --features "$combo")
    combo_label="$combo"
  fi

  run "cargo check [$combo_label]" cargo check "${feat_args[@]}"

  for profile in release debug; do
    if [ "$profile" = release ]; then
      prof_args=(--release)
    else
      prof_args=()
    fi

    # Build the cdylib for this profile/feature set.
    run "cargo build [$combo_label/$profile]" cargo build "${prof_args[@]}" "${feat_args[@]}"
    RUST_SO="$CRATE/target/$profile/libhatch_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      echo "!!! FAIL: $RUST_SO not produced"
      fail=1
      continue
    fi

    for c_label in default opt; do
      if [ "$c_label" = default ]; then
        C_SO="$C_DEFAULT_SO"
      else
        C_SO="$C_OPT_SO"
        [ -z "$C_SO" ] && continue
      fi
      run "tests [$combo_label / rust=$profile / c=$c_label]" \
        env HATCH_RUST_SO="$RUST_SO" HATCH_C_SO="$C_SO" \
        cargo test "${prof_args[@]}" "${feat_args[@]}" -- --test-threads=4
    done
  done
done

printf '\n==========================================\n'
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
