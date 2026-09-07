#!/usr/bin/env bash
# Differential test driver: builds the C .so, builds the Rust cdylib, then runs
# the Phase B/C/D differential suites for every feature combination.
#
# `cargo test` alone is NOT enough: this crate is `crate-type = ["cdylib"]`, and
# integration tests do not link against a cdylib, so cargo will happily run the
# suite against a stale `target/<profile>/libdriver.so`.  The suite has a
# staleness guard for exactly that, and this script does the rebuild.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

CARGO_FLAGS=(--offline)
PROFILE_FLAG=(--release)

echo "==> building C shared library"
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
ls -l "$root/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# Feature combinations.  `Cargo.toml` declares no [features], so the only
# combination is the default one; the loop below is derived from the manifest so
# it keeps working if features are ever added.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  cd "$here" && cargo metadata "${CARGO_FLAGS[@]}" --format-version 1 --no-deps \
    | python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["packages"][0]["features"]))' \
    | grep -v '^$' || true
)

declare -a COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  COMBOS+=("")                        # default build only
  COMBOS+=("--no-default-features")   # identical here, but exercised anyway
else
  COMBOS+=("")
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

status=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "############################################################"
  echo "# feature combination: $label"
  echo "############################################################"
  # shellcheck disable=SC2086
  (cd "$here" \
    && cargo build "${CARGO_FLAGS[@]}" "${PROFILE_FLAG[@]}" $combo \
    && cargo test "${CARGO_FLAGS[@]}" "${PROFILE_FLAG[@]}" $combo -- --nocapture) || status=1
done

echo
echo "############################################################"
echo "# extra configuration: unoptimised (dev) profile cdylib"
echo "#   the release build's optimiser is what removes bounds checks, so the"
echo "#   dev-profile .so is a genuinely different code path and is tested too"
echo "############################################################"
(cd "$here" \
  && cargo build "${CARGO_FLAGS[@]}" \
  && RUST_DRIVER_SO="$here/target/debug/libdriver.so" \
  cargo test "${CARGO_FLAGS[@]}" "${PROFILE_FLAG[@]}" -- --nocapture) || status=1

echo
echo "==> dev-profile symbol parity"
diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
  <(nm -D --defined-only "$here/target/debug/libdriver.so" | awk '{print $NF}' | sort) \
  && echo "dev symbol diff: EMPTY (parity OK)" || status=1

echo
echo "==> symbol parity"
diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
  <(nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print $NF}' | sort) \
  && echo "symbol diff: EMPTY (parity OK)" || status=1

exit "$status"
