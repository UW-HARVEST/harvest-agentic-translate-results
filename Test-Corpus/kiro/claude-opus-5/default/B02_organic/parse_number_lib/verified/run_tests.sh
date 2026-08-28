#!/usr/bin/env bash
# Full differential verification: build the C .so, build the Rust cdylib fresh,
# then run the FFI comparison suite for every feature combination.
set -uo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"
MODE="${1:-}"   # optional: "release"

status=0

# ---- 1. C shared library ----------------------------------------------------
echo ">>> Building C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
  timeout 600 cmake --build .
) > /tmp/c_build.log 2>&1 || { echo "!!! C build failed"; tail -30 /tmp/c_build.log; exit 1; }
echo "    $ROOT/c_src/build/libdriver.so"

# ---- 2. Enumerate feature combinations --------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1] }
  ' Cargo.toml
)
echo ">>> Features declared in Cargo.toml: ${FEATURES[*]:-<none>}"

COMBO_ARGS=()   # each entry is a space-separated arg list
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBO_ARGS+=("--no-default-features")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then
        [ -n "$combo" ] && combo="$combo,"
        combo="$combo${FEATURES[b]}"
      fi
    done
    if [ -z "$combo" ]; then
      COMBO_ARGS+=("--no-default-features")
    else
      COMBO_ARGS+=("--no-default-features --features $combo")
    fi
  done
fi
COMBO_ARGS+=("")              # default features
COMBO_ARGS+=("--all-features")

# ---- 3. Per-combination: rebuild cdylib, then run tests ---------------------
for args in "${COMBO_ARGS[@]}"; do
  label="${args:-<default features>}"
  echo "=============================================================="
  echo ">>> Configuration: $label"

  # Build the cdylib into a dedicated target dir so it cannot be stale and does
  # not fight `cargo test` for the main target-dir lock.
  SO_DIR="target/so/$(echo "$label" | tr -c 'A-Za-z0-9._-' '_')"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release --target-dir "$SO_DIR" $args > /tmp/so_build.log 2>&1; then
    echo "!!! cdylib build failed for $label"; tail -30 /tmp/so_build.log; status=1; continue
  fi
  SO="$(pwd)/$SO_DIR/release/libdriver.so"
  [ -f "$SO" ] || { echo "!!! no libdriver.so produced for $label"; status=1; continue; }
  echo "    cdylib: $SO"

  # shellcheck disable=SC2086
  if [ "$MODE" = "release" ]; then
    DRIVER_RUST_SO="$SO" timeout 600 cargo test --release $args > /tmp/test.log 2>&1
  else
    DRIVER_RUST_SO="$SO" timeout 600 cargo test $args > /tmp/test.log 2>&1
  fi
  if [ $? -ne 0 ]; then
    echo "!!! TESTS FAILED for $label"
    grep -E "^(test |error|thread |assertion|  C   |  Rust|mismatch)" /tmp/test.log | head -40
    tail -30 /tmp/test.log
    status=1
  else
    grep -E "^test result:" /tmp/test.log
  fi

  # ---- Symbol parity, checked directly on the built artifacts ----
  c_syms=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '$2 ~ /^[TtDBRWi]$/ {print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$SO" | awk '$2 ~ /^[TtDBRWi]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "!!! Rust .so missing C-exported symbols for $label:"; echo "$missing"; status=1
  else
    echo "    symbol parity OK ($(echo "$c_syms" | wc -l) C exports all present)"
  fi
done

echo "=============================================================="
if [ $status -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $status
