#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every valid
# Cargo feature combination.
#
# Usage: ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
FAIL=0

# ---------------------------------------------------------------------------
# 1. Build the C ground-truth shared library.
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > /tmp/verify-cmake.log 2>&1 \
  || { echo "FAIL: C build (see /tmp/verify-cmake.log)"; exit 1; }
C_SO="$ROOT/c_src/build/libhello.so"
echo "C library: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate every feature combination from translation/Cargo.toml.
#    The crate declares no [features], so the only combination is the empty
#    one. The power set is computed anyway so the script keeps working if
#    features are added later.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "="); gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
    }
  ' translation/Cargo.toml
)

COMBOS=("")
for f in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [[ -z "$existing" ]]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done

echo "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - [${c:-<none>}]"; done

# ---------------------------------------------------------------------------
# 3. For each combination: cargo check, cargo test (differential vs C),
#    and a dynamic-symbol export comparison.
# ---------------------------------------------------------------------------
cd translation
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    ARGS=(--no-default-features)
    LABEL="<none>"
  else
    ARGS=(--no-default-features --features "$combo")
    LABEL="$combo"
  fi
  SLUG="${LABEL//[^A-Za-z0-9]/_}"

  echo
  echo "===== features: $LABEL ====="

  for step in check test; do
    for profile in "" --release; do
      log="/tmp/verify-${step}-${SLUG}${profile:+-release}.log"
      extra=()
      [[ "$step" == check ]] && extra=(--all-targets)
      if timeout 600 cargo "$step" "${ARGS[@]}" ${profile:+$profile} "${extra[@]}" \
           > "$log" 2>&1; then
        echo "  PASS  cargo $step ${profile:-(debug)}"
      else
        echo "  FAIL  cargo $step ${profile:-(debug)}  -> $log"
        tail -n 25 "$log" | sed 's/^/        /'
        FAIL=1
      fi
    done
  done

  # Symbol comparison: every symbol exported by the C .so must also be
  # exported by the Rust .so, for both profiles.
  for profile in "" --release; do
    prof_dir=debug; [[ -n "$profile" ]] && prof_dir=release
    timeout 600 cargo build "${ARGS[@]}" ${profile:+$profile} --lib \
      --target-dir target/ffi-cdylib > "/tmp/verify-build-${SLUG}-${prof_dir}.log" 2>&1 \
      || { echo "  FAIL  cargo build ($prof_dir) for symbol check"; FAIL=1; continue; }
    RUST_SO="$ROOT/translation/target/ffi-cdylib/$prof_dir/libhello.so"

    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [[ -z "$missing" ]]; then
      echo "  PASS  exported symbols ($prof_dir): all C exports present in Rust .so"
    else
      echo "  FAIL  Rust .so ($prof_dir) is missing C exports:"
      echo "$missing" | sed 's/^/        /'
      FAIL=1
    fi
  done
done

echo
if [[ $FAIL -eq 0 ]]; then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "FAILURES DETECTED"
fi
exit $FAIL
