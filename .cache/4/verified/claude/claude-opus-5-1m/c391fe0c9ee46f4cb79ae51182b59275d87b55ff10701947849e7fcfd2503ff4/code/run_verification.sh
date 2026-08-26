#!/usr/bin/env bash
# Full differential-verification run: builds the C reference .so, then runs every
# test phase against every Cargo feature combination and both build profiles.
#
# `Cargo.toml` declares no [features], so the feature combination set is exactly
# {"" (default == no-default-features)}. The list is derived mechanically below
# rather than hard-coded, so adding a feature later widens the matrix
# automatically.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CARGO_FLAGS="--offline"
FAIL=0

# --------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml.
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml
)
echo "== features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]-none}) =="

COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "== feature combinations to verify: ${#COMBOS[@]} =="

# --------------------------------------------------------------------------
# 2. Build the C reference shared library.
# --------------------------------------------------------------------------
echo "== building the C reference .so =="
(
  mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || {
  echo "!! C build failed"
  exit 1
}
ls -l c_src/build/libtranslated_rust.so

# --------------------------------------------------------------------------
# 3. cargo check + build + test for every combination and profile.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FEAT_ARGS=(--no-default-features)
    label="<no-default-features>"
  else
    FEAT_ARGS=(--no-default-features --features "$combo")
    label="$combo"
  fi

  for profile in debug release; do
    PROF_ARGS=()
    [ "$profile" = release ] && PROF_ARGS=(--release)

    echo
    echo "=============================================================="
    echo "== features: $label   profile: $profile"
    echo "=============================================================="

    if ! timeout 600 cargo check $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" --all-targets 2>&1 | tail -5; then
      echo "!! cargo check FAILED ($label/$profile)"
      FAIL=1
      continue
    fi

    # The cdylib must be rebuilt explicitly: `cargo test` does not refresh a
    # cdylib that the test crates do not link against.
    if ! timeout 600 cargo build $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" 2>&1 | tail -3; then
      echo "!! cargo build FAILED ($label/$profile)"
      FAIL=1
      continue
    fi

    if ! timeout 600 cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" -- --test-threads=4 2>&1 |
      grep -E "^(test result|running|error|test .* FAILED|---- )"; then
      echo "!! cargo test reported no summary ($label/$profile)"
      FAIL=1
    fi
    if [ "${PIPESTATUS[0]}" != 0 ]; then
      echo "!! cargo test FAILED ($label/$profile)"
      FAIL=1
    fi
  done
done

# --------------------------------------------------------------------------
# 4. Robustness cross-check: the same suite against optimized builds of the C
#    reference. A divergence here would mean the Rust matches only the exact
#    code gcc happens to emit at -O0 for some undefined-behaviour input.
# --------------------------------------------------------------------------
TMP="${TMPDIR:-/tmp}"
for lvl in O0 O2 O3; do
  so="$TMP/libc_ref_$lvl.so"
  if ! gcc "-$lvl" -shared -fPIC -Ic_src/include -o "$so" c_src/src/lib.c 2>/dev/null; then
    echo "== skipping C -$lvl cross-check (gcc unavailable) =="
    continue
  fi
  echo
  echo "== cross-check: C reference built with -$lvl =="
  if ! C_SO_PATH="$so" timeout 600 cargo test $CARGO_FLAGS --no-default-features 2>&1 |
    grep -E "^(test result|error|test .* FAILED)"; then
    echo "!! cross-check -$lvl FAILED"
    FAIL=1
  fi
  [ "${PIPESTATUS[0]}" != 0 ] && {
    echo "!! cross-check -$lvl FAILED"
    FAIL=1
  }
  rm -f "$so"
done

echo
if [ "$FAIL" = 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
