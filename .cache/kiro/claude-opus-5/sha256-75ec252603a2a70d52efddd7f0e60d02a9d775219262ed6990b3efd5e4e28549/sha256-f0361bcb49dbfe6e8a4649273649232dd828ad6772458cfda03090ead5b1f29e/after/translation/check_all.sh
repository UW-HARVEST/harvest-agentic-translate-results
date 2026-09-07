#!/usr/bin/env bash
# Full verification driver.
#
#   ./check_all.sh
#
# 1. builds the C shared library
# 2. enumerates every Cargo feature combination and `cargo check`s each
# 3. builds the Rust cdylib and runs every test suite under each combination
#
# The malloc environment is pinned so that the places where the C reads
# uninitialised heap (a malformed PNG whose DEFLATE stream is shorter than
# (w+1)*h*bpp leaves the tail of `img.pix` untouched, yet `cp_unfilter` and
# `cp_convert` still walk all of it) produce a deterministic, identical value in
# both libraries. glibc's tcache fast path skips the perturbation, hence the
# tunable.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
export MALLOC_PERTURB_=42
export GLIBC_TUNABLES=glibc.malloc.tcache_count=0

fail=0
step() { echo; echo "=== $* ==="; }

step "build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "C .so: $C_SO"

step "enumerate feature combinations from Cargo.toml"
# Every subset of the declared features, plus the default and no-default builds.
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip()
        if name != 'default':
            feats.append(name)
for f in feats:
    print(f)
PY
)
# drop any empty entries so an absent [features] section yields NFEAT == 0

tmp=()
for f in ${FEATURES[@]+"${FEATURES[@]}"}; do [ -n "$f" ] && tmp+=("$f"); done
FEATURES=(${tmp[@]+"${tmp[@]}"})
NFEAT=${#FEATURES[@]}
echo "declared features: ${NFEAT} ${FEATURES[*]:-(none)}"
if grep -qE '^\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ]; then
  echo "NOTE: crate declares a binary target - stdout comparison required"
else
  echo "no binary target (no [[bin]] and no src/main.rs); C CMakeLists has no add_executable either"
fi

COMBOS=("")                          # default features
if [ "$NFEAT" -gt 0 ]; then
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
  total=$((1 << NFEAT))
  for ((mask=0; mask<total; mask++)); do
    sel=()
    for ((b=0; b<NFEAT; b++)); do
      if (( (mask >> b) & 1 )); then sel+=("${FEATURES[$b]}"); fi
    done
    if [ ${#sel[@]} -eq 0 ]; then continue; fi
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

step "cargo check every combination"
for flags in "${COMBOS[@]}"; do
  echo "--- cargo check ${flags:-<default>}"
  # shellcheck disable=SC2086
  timeout 600 cargo check $flags 2>&1 | tail -2 || fail=1
done

step "build + test every combination"
for flags in "${COMBOS[@]}"; do
  echo
  echo "########## combination: ${flags:-<default>}"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $flags >/dev/null 2>&1 || { echo "build FAILED"; fail=1; continue; }
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > /tmp/c.syms
  nm -D --defined-only target/release/libload_png_mem_lib.so | awk '{print $3}' | sort > /tmp/r.syms
  missing=$(comm -23 /tmp/c.syms /tmp/r.syms)
  if [ -n "$missing" ]; then
    echo "SYMBOL DIFF NOT EMPTY for ${flags:-<default>}:"; echo "$missing"; fail=1
  else
    echo "symbol diff: empty ($(wc -l < /tmp/c.syms) C symbols all present)"
  fi
  for t in symbols phase_b_inflate phase_b_png phase_c_errors isolated; do
    printf '  %-18s ' "$t"
    # shellcheck disable=SC2086
    out=$(timeout 600 cargo test --release $flags --test "$t" 2>&1)
    if echo "$out" | grep -qE 'test result: ok|isolated: ok'; then
      echo "ok  $(echo "$out" | grep -oE '[0-9]+ (passed|cases matched)' | tail -1)"
    else
      echo "FAILED"; echo "$out" | tail -25; fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
