#!/usr/bin/env bash
# Differential test runner.
#
# Two things this automates that are easy to get wrong by hand:
#   1. `cargo test` does NOT build the cdylib (crate-type = ["cdylib"]), so the
#      .so under test MUST be produced by an explicit `cargo build` first.
#      Otherwise the tests load a stale artifact and silently pass.
#   2. Feature combinations are enumerated from Cargo.toml rather than assumed.

set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"
FAILED=0

echo "=== Building the C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }

# --- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== Cargo.toml declares no [features]: single default configuration ==="
  COMBOS=("DEFAULT")
else
  COMBOS=("DEFAULT" "NONE")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  # full cross-product of the declared features
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

run_combo() {
  local label="$1" profile="$2"; shift 2
  local featflags=("$@")

  echo
  echo "############################################################"
  echo "### features: ${label}   profile: ${profile}"
  echo "############################################################"

  local profflag=()
  local outdir="target/debug"
  if [ "$profile" = "release" ]; then profflag=(--release); outdir="target/release"; fi

  # (1) build the cdylib so a FRESH .so exists for the harness to dlopen
  if ! cargo build $CARGO_FLAGS "${profflag[@]}" "${featflags[@]}" 2>&1 | tail -3; then
    echo "BUILD FAILED for [$label/$profile]"; FAILED=1; return
  fi

  DRIVER_RUST_SO="$(pwd)/${outdir}/libdriver.so"
  if [ ! -f "$DRIVER_RUST_SO" ]; then
    echo "MISSING cdylib at $DRIVER_RUST_SO"; FAILED=1; return
  fi
  export DRIVER_RUST_SO

  # (2) symbol parity for this configuration
  echo "--- symbol diff (C -> Rust) ---"
  local missing
  missing=$(comm -23 \
    <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$DRIVER_RUST_SO"           | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING SYMBOLS in Rust .so for [$label/$profile]:"; echo "$missing"; FAILED=1
  else
    echo "OK: 0 C symbols missing from the Rust .so"
  fi

  # (3) the differential tests
  if ! timeout 600 cargo test $CARGO_FLAGS "${profflag[@]}" "${featflags[@]}" 2>&1 \
        | grep -E '^test |test result:|panicked|DIVERGENCE|error(\[|:)'; then
    :
  fi
  # shellcheck disable=SC2181
  if ! timeout 600 cargo test $CARGO_FLAGS "${profflag[@]}" "${featflags[@]}" >/dev/null 2>&1; then
    echo "TESTS FAILED for [$label/$profile]"; FAILED=1
  fi
  unset DRIVER_RUST_SO
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) flags=() ;;
    NONE)    flags=(--no-default-features) ;;
    *)       flags=(--no-default-features --features "$combo") ;;
  esac
  for profile in debug release; do
    run_combo "$combo" "$profile" "${flags[@]}"
  done
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "=================== ALL CONFIGURATIONS PASSED ==================="
else
  echo "=================== FAILURES PRESENT ==========================="
fi
exit "$FAILED"
