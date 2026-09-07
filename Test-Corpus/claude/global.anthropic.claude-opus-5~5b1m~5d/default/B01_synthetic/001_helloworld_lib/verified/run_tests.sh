#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run every differential test in every
# feature combination. Tests MUST run serially (they redirect process-wide fd 1).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_OFFLINE="${CARGO_OFFLINE:---offline}"

echo "== building C shared library =="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
export C_HELLO_SO="$ROOT/c_src/build/libhello.so"
ls -l "$C_HELLO_SO"

# Feature combinations declared in Cargo.toml ([features] section). If there are
# none, the single "default" configuration is the whole matrix.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[ \t]*=/ {gsub(/[ \t].*/,""); if ($0 != "default") print}
' "$HERE/Cargo.toml")

declare -a COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("default")
else
  COMBOS=("default" "none")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  # all features together
  COMBOS+=("$(printf '%s,' "${FEATURES[@]}" | sed 's/,$//')")
fi

status=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "=================================================================="
  echo "== feature combination: $combo"
  echo "=================================================================="
  case "$combo" in
    default) FLAGS=() ;;
    none)    FLAGS=(--no-default-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac

  echo "-- cargo build --release ${FLAGS[*]:-}"
  (cd "$HERE" && cargo build $CARGO_OFFLINE --release "${FLAGS[@]}" >/dev/null)
  export RUST_HELLO_SO="$HERE/target/release/libhello.so"
  ls -l "$RUST_HELLO_SO"

  echo "-- symbol diff (C symbols missing from Rust) --"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_HELLO_SO"  | awk '$2 ~ /^[TDBRWViu]$/ {print $3}' | sort -u) \
    <(nm -D --defined-only "$RUST_HELLO_SO" | awk '$2 ~ /^[TDBRWViu]$/ {print $3}' | sort -u) \
    | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__gmon_start__|_ITM_|__cxa_|__gnu_)' || true)
  if [ -n "$missing" ]; then
    echo "MISSING SYMBOLS:"; echo "$missing"; status=1
  else
    echo "(empty — full parity)"
  fi

  echo "-- cargo test (serial) --"
  if ! (cd "$HERE" && RUST_TEST_THREADS=1 cargo test $CARGO_OFFLINE "${FLAGS[@]}" -- --test-threads=1); then
    status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$status"
