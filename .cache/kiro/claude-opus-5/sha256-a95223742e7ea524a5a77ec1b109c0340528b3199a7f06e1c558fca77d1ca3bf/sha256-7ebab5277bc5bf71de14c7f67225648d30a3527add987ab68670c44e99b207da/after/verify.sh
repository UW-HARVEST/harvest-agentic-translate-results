#!/usr/bin/env bash
# Build the C shared library and run the differential test suite for every
# valid feature combination declared in translation/Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR=/tmp/xlat-verify
mkdir -p "$LOG_DIR"

# ---- 1. build the C reference -------------------------------------------------
echo "== building C reference =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOG_DIR/c-build.log" 2>&1 \
  || { echo "C build FAILED"; tail -30 "$LOG_DIR/c-build.log"; exit 1; }

# ---- 2. enumerate feature combinations ---------------------------------------
# Parse the [features] table of Cargo.toml; ignore "default" and any
# `dep:`/optional-dependency implicit features.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock=1; next }
    /^\[/           { inblock=0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$ROOT/translation/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("")   # no features declared: the empty combination is the only one
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]@Q} =="

# ---- 3. check + test every combination ---------------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"

  for profile in "" "--release"; do
    pslug="${profile:---debug}"
    echo "-- features=$label profile=${profile:-debug}"

    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ -n "$profile" ] && args+=("$profile")

    if ! ( cd "$ROOT/translation" && timeout 600 cargo check "${args[@]}" ) \
        > "$LOG_DIR/check-$slug$pslug.log" 2>&1; then
      echo "   cargo check FAILED"; tail -30 "$LOG_DIR/check-$slug$pslug.log"; FAIL=1; continue
    fi

    # The tests dlopen the cdylib, so it must exist for this profile/feature set.
    if ! ( cd "$ROOT/translation" && timeout 600 cargo build "${args[@]}" ) \
        > "$LOG_DIR/build-$slug$pslug.log" 2>&1; then
      echo "   cargo build FAILED"; tail -30 "$LOG_DIR/build-$slug$pslug.log"; FAIL=1; continue
    fi

    if ! ( cd "$ROOT/translation" && timeout 600 cargo test "${args[@]}" ) \
        > "$LOG_DIR/test-$slug$pslug.log" 2>&1; then
      echo "   cargo test FAILED"; tail -40 "$LOG_DIR/test-$slug$pslug.log"; FAIL=1; continue
    fi

    grep -h "test result:" "$LOG_DIR/test-$slug$pslug.log" | sed 's/^/   /'
  done
done

# ---- 4. symbol parity --------------------------------------------------------
echo "== nm -D symbol comparison =="
diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u) \
     <(nm -D --defined-only "$ROOT/translation/target/release/libdriver.so" | awk '{print $NF}' | sort -u) \
     && echo "   identical exported symbol sets"

[ "$FAIL" -eq 0 ] && echo "== ALL COMBINATIONS PASS ==" || echo "== FAILURES PRESENT =="
exit "$FAIL"
