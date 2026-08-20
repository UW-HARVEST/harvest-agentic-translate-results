#!/usr/bin/env bash
# Full verification run: build the C reference .so, build the Rust cdylib for
# every feature combination, compare exported symbols, then run the differential
# test suite.
#
# NOTE: `cargo test` does NOT rebuild a `cdylib`-only lib target, so the
# `cargo build` before each `cargo test` is mandatory. The test harness also
# refuses to run against a `.so` older than `src/`, as a backstop.
set -uo pipefail
cd "$(dirname "$0")"

fail=0

echo "=== 1. build the C reference shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
C_SO=$(ls c_src/build/*.so | head -1)
echo "C .so: $C_SO"

# Cargo.toml has no [features] section, so the only combination is the empty
# one. Enumerate it from the file anyway so the loop keeps working if features
# are ever added.
mapfile -t COMBOS < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { gsub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml
)
if [ "${#COMBOS[@]}" -eq 0 ]; then
  COMBOS=("")
  echo "=== no [features] in Cargo.toml -> single combination (empty) ==="
else
  echo "=== features found: ${COMBOS[*]} ==="
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo
  echo "############################################################"
  echo "### feature combination: $label"
  echo "############################################################"

  if [ -z "$combo" ]; then
    FEAT=(--no-default-features)
  else
    FEAT=(--no-default-features --features "$combo")
  fi

  echo "--- cargo check ---"
  cargo check "${FEAT[@]}" 2>&1 | tail -3 || fail=1

  echo "--- cargo build (produces the cdylib the tests dlopen) ---"
  cargo build "${FEAT[@]}" 2>&1 | tail -3 || { echo "RUST BUILD FAILED"; fail=1; continue; }
  R_SO=target/debug/libencode_quant_lib.so

  echo "--- symbol parity (nm -D) ---"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
  else
    echo "OK: every symbol exported by the C .so is exported by the Rust .so"
  fi
  undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
          | grep -vE '@|^_ITM_|^__gmon_start__$|^_Unwind' || true)
  if [ -n "$undef" ]; then
    echo "UNRESOLVED NON-LIBC SYMBOLS IN RUST .so:"; echo "$undef"; fail=1
  else
    echo "OK: no unresolved non-libc symbols in the Rust .so"
  fi

  echo "--- cargo test (Phase B + Phase C) ---"
  cargo test "${FEAT[@]}" --no-fail-fast 2>&1 | grep -E "^test |test result|DIVERGENCE|STALE" || fail=1
  cargo test "${FEAT[@]}" --no-fail-fast >/dev/null 2>&1 || fail=1
done

echo
if [ "$fail" -eq 0 ]; then
  echo "############## ALL PHASES PASSED ##############"
else
  echo "############## FAILURES PRESENT ##############"
fi
exit "$fail"
