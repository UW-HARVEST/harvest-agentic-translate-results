#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination and
# under both build profiles. Feature names are extracted from Cargo.toml rather
# than hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/*.so 2>/dev/null | head -1)
if [ -z "${C_SO}" ]; then
  echo "FATAL: C shared library not built. Run:"
  echo "  cd ../c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi
echo "C .so: ${C_SO}"

# --- enumerate features declared in Cargo.toml -------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inb=1; next }
    /^\[/           { inb=0 }
    inb && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the combination list ---------------------------------------------
# Always cover: default, --no-default-features, --all-features, plus the full
# power set of declared non-"default" features.
COMBOS=("--" "--no-default-features" "--all-features")
NONDEF=()
for f in "${FEATURES[@]:-}"; do [ -n "$f" ] && [ "$f" != "default" ] && NONDEF+=("$f"); done
n=${#NONDEF[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="${sel:+$sel,}${NONDEF[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
    COMBOS+=("--features $sel")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

FAIL=0
for profile in release debug; do
  PROF_FLAG=""
  [ "$profile" = "release" ] && PROF_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    # "--" is the sentinel for "no feature flags at all" (the default build).
    if [ "$combo" = "--" ]; then flags=""; else flags="$combo"; fi
    label="profile=${profile} features='${flags:-<default>}'"

    echo "=============================================================="
    echo ">>> cargo check   ${label}"
    if ! timeout 600 cargo check $PROF_FLAG --all-targets $flags >/tmp/fc_check.log 2>&1; then
      echo "!!! cargo check FAILED (${label})"; tail -20 /tmp/fc_check.log; FAIL=1; continue
    fi

    echo ">>> cargo build   ${label}"
    if ! timeout 600 cargo build $PROF_FLAG $flags >/tmp/fc_build.log 2>&1; then
      echo "!!! cargo build FAILED (${label})"; tail -20 /tmp/fc_build.log; FAIL=1; continue
    fi

    # Make the harness pick the .so for THIS profile by hiding the other one.
    OTHER=release; [ "$profile" = "release" ] && OTHER=debug
    HIDDEN=""
    if [ "$profile" = "debug" ] && [ -f target/release/libbin2hex_lib.so ]; then
      mv target/release/libbin2hex_lib.so target/release/libbin2hex_lib.so.hidden
      HIDDEN=target/release/libbin2hex_lib.so
    fi
    if [ ! -f "target/${profile}/libbin2hex_lib.so" ]; then
      echo "!!! missing target/${profile}/libbin2hex_lib.so (${label})"; FAIL=1
      [ -n "$HIDDEN" ] && mv "${HIDDEN}.hidden" "$HIDDEN"
      continue
    fi
    echo "    using $(readlink -f target/${profile}/libbin2hex_lib.so)"

    echo ">>> cargo test    ${label}"
    timeout 600 cargo test $PROF_FLAG $flags -- --test-threads=1 >/tmp/fc_test.log 2>&1
    rc=$?
    [ -n "$HIDDEN" ] && mv "${HIDDEN}.hidden" "$HIDDEN"
    grep -E '^test result|^running|Running' /tmp/fc_test.log
    if [ $rc -ne 0 ]; then
      echo "!!! cargo test FAILED rc=$rc (${label})"
      grep -E 'FAILED|panicked|^---- ' /tmp/fc_test.log | head -40
      FAIL=1
    fi
  done
done

echo "=============================================================="
if [ $FAIL -eq 0 ]; then echo "ALL FEATURE COMBINATIONS x PROFILES PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $FAIL
