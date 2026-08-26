#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination, cargo check it, then run the
# full differential test suite (Phase B + Phase C) for each one.
#
# Cargo.toml declares no [features], so the powerset of features is exactly one
# element: the empty set. Both spellings (`--no-default-features` and the plain
# default build) are exercised so the invocation stays correct if features are
# added later.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
LOGDIR=${TMPDIR:-/tmp}/diffverify
mkdir -p "$LOGDIR"

# --- feature enumeration (mechanical, from Cargo.toml) ----------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)
echo "declared non-default features: ${#FEATURES[@]} -> ${FEATURES[*]:-<none>}"

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
  done
  COMBOS+=("$combo")
done
# always also test the default feature set
COMBOS+=("__default__")

# --- ensure the C oracle exists --------------------------------------------
if ! ls c_src/build/lib*.so >/dev/null 2>&1; then
  echo "building C shared library..."
  (mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .) >"$LOGDIR/cmake.log" 2>&1 ||
    { echo "C build FAILED, see $LOGDIR/cmake.log"; exit 1; }
fi
echo "C oracle: $(ls "$ROOT"/c_src/build/lib*.so)"

status=0
for combo in "${COMBOS[@]}"; do
  if [[ $combo == "__default__" ]]; then
    args=()
    label="default"
  else
    args=(--no-default-features)
    [[ -n $combo ]] && args+=(--features "$combo")
    label="no-default-features${combo:+ +$combo}"
  fi

  log="$LOGDIR/$(echo "$label" | tr ' +,' '___').log"

  echo "=== cargo check [$label] ==="
  if ! timeout 600 cargo check --all-targets "${args[@]}" >"$log" 2>&1; then
    echo "  CHECK FAILED ($log)"; tail -30 "$log"; status=1; continue
  fi
  grep -c '^warning' "$log" >/dev/null && echo "  check ok ($(grep -c '^warning' "$log") warnings)"

  # the cdylib is not produced by `cargo test`, so build it explicitly: the
  # tests dlopen it instead of linking it.
  echo "=== cargo build [$label] (cdylib for dlopen) ==="
  if ! timeout 600 cargo build --release "${args[@]}" >>"$log" 2>&1; then
    echo "  BUILD FAILED ($log)"; tail -30 "$log"; status=1; continue
  fi

  echo "=== cargo test  [$label] ==="
  if ! timeout 600 cargo test --release "${args[@]}" >>"$log" 2>&1; then
    echo "  TESTS FAILED ($log)"; grep -E 'panicked|FAILED|test result|divergence' "$log" | tail -30; status=1; continue
  fi
  grep -E 'test result' "$log" | sed 's/^/  /'

  # --- symbol parity for this configuration ---
  rust_so=$(ls target/release/libdataentry_lib.so 2>/dev/null || true)
  c_so=$(ls c_src/build/lib*.so | head -1)
  if [[ -z $rust_so ]]; then
    echo "  MISSING rust .so"; status=1; continue
  fi
  missing=$(comm -23 \
    <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))
  if [[ -n $missing ]]; then
    echo "  SYMBOL PARITY FAILED, missing from Rust .so:"; echo "$missing" | sed 's/^/    /'; status=1
  else
    echo "  symbol parity OK ($(nm -D --defined-only "$c_so" | wc -l) C symbols all present)"
  fi
done

echo
if ((status == 0)); then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $status
