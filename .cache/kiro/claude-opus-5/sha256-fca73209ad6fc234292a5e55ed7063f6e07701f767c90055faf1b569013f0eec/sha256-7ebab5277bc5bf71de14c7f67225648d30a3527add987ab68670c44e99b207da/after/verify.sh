#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build-time
# feature combination declared in translation/Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR="/tmp/xlate-verify"
mkdir -p "$LOGDIR"
rc=0
cd "$CRATE" || exit 1

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml [features]
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default") print a[1] }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("")            # no [features] table: the default build is the only one
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

step "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  - ${c:-<none / default>}"; done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library
# ---------------------------------------------------------------------------
step "building C reference"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) >"$LOGDIR/cmake.log" 2>&1 || { echo "C build FAILED (see $LOGDIR/cmake.log)"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 3-4. For each combination: cargo check, build both profiles, run tests
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FLAGS=()
    tag="default"
  else
    FLAGS=(--no-default-features --features "$combo")
    tag="$combo"
  fi
  safe_tag="${tag//,/_}"

  step "combo: $tag -- cargo check"
  if ! timeout 600 cargo check "${FLAGS[@]}" --all-targets \
    >"$LOGDIR/check-$safe_tag.log" 2>&1; then
    echo "  CHECK FAILED"; tail -30 "$LOGDIR/check-$safe_tag.log"; rc=1; continue
  fi
  echo "  ok"

  for profile in debug release; do
    step "combo: $tag -- cargo build ($profile) + nm parity"
    build=("${FLAGS[@]}")
    [ "$profile" = release ] && build+=(--release)
    if ! timeout 600 cargo build "${build[@]}" \
      >"$LOGDIR/build-$safe_tag-$profile.log" 2>&1; then
      echo "  BUILD FAILED"; tail -30 "$LOGDIR/build-$safe_tag-$profile.log"; rc=1; continue
    fi
    R_SO="$CRATE/target/$profile/libmaxnmin_lib.so"
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -v '^_' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^_' | sort -u))"
    if [ -n "$missing" ]; then
      echo "  MISSING SYMBOLS in $R_SO:"; echo "$missing" | sed 's/^/    /'; rc=1
    else
      echo "  symbol parity ok ($profile)"
    fi
  done

  for profile in debug release; do
    step "combo: $tag -- cargo test ($profile)"
    t=("${FLAGS[@]}")
    [ "$profile" = release ] && t+=(--release)
    if timeout 600 cargo test "${t[@]}" >"$LOGDIR/test-$safe_tag-$profile.log" 2>&1; then
      grep -h '^test result:' "$LOGDIR/test-$safe_tag-$profile.log" | sed 's/^/  /'
    else
      echo "  TESTS FAILED (see $LOGDIR/test-$safe_tag-$profile.log)"
      grep -E '^(test |thread |error|assertion|  left|  right)' \
        "$LOGDIR/test-$safe_tag-$profile.log" | tail -40 | sed 's/^/  /'
      rc=1
    fi
  done
done

step "RESULT"
[ "$rc" -eq 0 ] && echo "ALL COMBINATIONS PASS" || echo "FAILURES PRESENT"
exit "$rc"
