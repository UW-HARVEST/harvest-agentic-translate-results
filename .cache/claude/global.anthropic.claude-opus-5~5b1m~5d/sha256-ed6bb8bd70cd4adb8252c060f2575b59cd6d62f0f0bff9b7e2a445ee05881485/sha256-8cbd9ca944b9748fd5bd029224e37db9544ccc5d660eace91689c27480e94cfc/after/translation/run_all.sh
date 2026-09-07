#!/usr/bin/env bash
# Full verification run: build both libraries, diff their dynamic symbol
# tables, and run the Phase B/C/D differential suites under EVERY feature
# combination declared in Cargo.toml.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
C_SO="$ROOT/c_src/build/libdriver.so"
RUST_SO="$HERE/target/release/libdriver.so"
# crates.io is unreachable in this environment; the deps are in the local
# registry cache, so resolve offline.
OFFLINE="--offline"
rc=0

echo "=== 1. build C shared library ==========================================="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || rc=1
ls -l "$C_SO" || rc=1

echo
echo "=== 2. build Rust cdylib (release) ======================================="
( cd "$HERE" && cargo build --release $OFFLINE ) || rc=1
ls -l "$RUST_SO" || rc=1

echo
echo "=== 3. Phase D symbol diff (must be empty) =============================="
syms() { nm -D --defined-only "$1" | awk '{print $NF}' \
         | grep -vE '^(_init|_fini|__bss_start|_edata|_end|_ITM_.*)$' | sort -u; }
diff <(syms "$C_SO") <(syms "$RUST_SO") && echo "SYMBOL DIFF EMPTY: OK" || {
  echo "SYMBOL DIFF NON-EMPTY: FAIL"; rc=1; }

echo
echo "=== 4. binary/driver executable check ==================================="
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C builds an executable -- stdout comparison required"; rc=1
else
  echo "no add_executable in CMakeLists.txt"
fi
if [ -f "$HERE/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$HERE/Cargo.toml"; then
  echo "Rust declares a binary -- stdout comparison required"; rc=1
else
  echo "no [[bin]] / src/main.rs in the Rust crate"
fi
echo "=> no driver binary on either side; binary-stdout obligation is vacuous"

echo
echo "=== 5. enumerate feature combinations ==================================="
# Every feature declared in [features], if any.
FEATURES=$(cd "$HERE" && python3 - <<'PY'
import re
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*(.*?)(?=^\[|\Z)', txt, re.S | re.M)
if not m:
    print("", end="")
else:
    names = [l.split('=')[0].strip() for l in m.group(1).splitlines()
             if '=' in l and not l.strip().startswith('#')]
    print(" ".join(n for n in names if n != "default"), end="")
PY
)
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] -> exactly one configuration."
  COMBOS=("default" "no-default-features")
else
  echo "features: $FEATURES"
  COMBOS=("default" "no-default-features")
  for f in $FEATURES; do COMBOS+=("$f"); done
  # full power set
  set -- $FEATURES
  n=$#
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then eval "v=\${$((i+1))}"; combo="$combo,$v"; fi
    done
    COMBOS+=("${combo#,}")
  done
fi

echo
echo "=== 6. run the differential suites for every combination ================"
for combo in "${COMBOS[@]}"; do
  echo
  echo "--- combination: $combo ---"
  case "$combo" in
    default)              ARGS=() ;;
    no-default-features)  ARGS=(--no-default-features) ;;
    *)                    ARGS=(--no-default-features --features "$combo") ;;
  esac
  ( cd "$HERE" \
    && cargo build --release $OFFLINE "${ARGS[@]}" \
    && timeout 600 cargo test $OFFLINE "${ARGS[@]}" -- --nocapture ) \
      2>&1 | grep -E '^\s+\[(PASS|FAIL)\]|^test result|^error'
  # shellcheck disable=SC2181
  if [ "${PIPESTATUS[0]}" -ne 0 ]; then
    echo "COMBINATION $combo: FAIL"; rc=1
  else
    echo "COMBINATION $combo: OK"
  fi
done

echo
if [ "$rc" -eq 0 ]; then
  echo "=== ALL PHASES PASSED ==="
else
  echo "=== FAILURES PRESENT (rc=$rc) ==="
fi
exit "$rc"
