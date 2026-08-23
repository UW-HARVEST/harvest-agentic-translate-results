#!/usr/bin/env bash
# Full verification driver: enumerates every Cargo feature combination,
# builds and checks each, rebuilds the C reference .so, compares exported
# symbols, runs the differential test suite, and diffs the C/Rust driver
# binaries' stdout.
#
# Usage: ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
GREP=/usr/bin/grep
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  [ OK ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- features
step "Enumerating Cargo feature combinations"
# Extract feature names from the [features] table (ignore the "default" key and
# any optional-dependency implicit features).
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares NO [features] table."
  echo "  => exactly ONE valid configuration exists (the default/empty feature set)."
  COMBOS=("")
else
  # shellcheck disable=SC2206
  FARR=($FEATURES)
  n=${#FARR[@]}
  echo "  features: ${FARR[*]}"
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  echo "  => ${#COMBOS[@]} combinations (2^$n)"
fi

step "cargo check for EVERY feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  if timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} \
       > /tmp/chk.log 2>&1; then
    ok "cargo check --no-default-features --features '$label'"
  else
    bad "cargo check --features '$label'"; tail -20 /tmp/chk.log
  fi
done

# ---------------------------------------------------------------- C reference
step "Building the C reference shared library"
mkdir -p cbuild
if (cd cbuild && cmake ../c_src -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /tmp/cm.log 2>&1 \
      && cmake --build . -j8 >> /tmp/cm.log 2>&1); then
  ok "cbuild/libjansson.so"
else
  bad "C build"; tail -20 /tmp/cm.log
fi

# ---------------------------------------------------------------- per combo
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  step "Feature combination: $label"

  if timeout 600 cargo build --release --no-default-features \
       ${combo:+--features "$combo"} > /tmp/bld.log 2>&1; then
    ok "cargo build --release"
  else
    bad "cargo build --release (features '$label')"; tail -20 /tmp/bld.log; continue
  fi

  # ---- symbol parity
  nm -D --defined-only cbuild/libjansson.so \
    | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only target/release/libjansson.so \
    | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > /tmp/r_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  nc=$(wc -l < /tmp/c_syms.txt); nr=$(wc -l < /tmp/r_syms.txt)
  if [ -z "$missing" ]; then
    ok "symbol parity: C=$nc Rust=$nr, 0 missing"
  else
    bad "missing symbols:"; echo "$missing" | sed 's/^/         /'
  fi
  # no unresolved jansson symbols (libc/unwind only)
  und=$(nm -D --undefined-only target/release/libjansson.so | awk '{print $2}' \
        | $GREP -E '^(json_|jsonp_|hashtable_|strbuffer_|utf8_|dtoa|jansson_)' || true)
  if [ -z "$und" ]; then
    ok "no unresolved jansson symbols"
  else
    bad "unresolved: $und"
  fi

  # ---- driver binary stdout comparison
  if gcc -O1 -o /tmp/diff_driver tests_c/diff_driver.c -ldl 2>/dev/null; then
    timeout 600 /tmp/diff_driver "$ROOT/cbuild/libjansson.so"          > /tmp/drv_c.txt 2>&1
    timeout 600 /tmp/diff_driver "$ROOT/target/release/libjansson.so"  > /tmp/drv_r.txt 2>&1
    if cmp -s /tmp/drv_c.txt /tmp/drv_r.txt; then
      ok "driver stdout byte-identical ($(wc -l < /tmp/drv_c.txt) lines)"
    else
      d=$(diff /tmp/drv_c.txt /tmp/drv_r.txt | $GREP -c '^[<>]')
      bad "driver stdout differs on $d lines"
      diff /tmp/drv_c.txt /tmp/drv_r.txt | head -10 | sed 's/^/         /'
    fi
  else
    bad "could not build tests_c/diff_driver.c"
  fi

  # ---- differential test suite
  if timeout 600 cargo test --release --no-default-features \
       ${combo:+--features "$combo"} > /tmp/tst.log 2>&1; then
    passed=$($GREP -Eo '[0-9]+ passed' /tmp/tst.log | awk '{s+=$1} END{print s+0}')
    ok "cargo test: $passed tests passed"
  else
    bad "cargo test (features '$label')"
    $GREP -E "^test .*FAILED|^error|divergence|panicked at" /tmp/tst.log \
      | head -25 | sed 's/^/         /'
  fi
done

step "RESULT"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
