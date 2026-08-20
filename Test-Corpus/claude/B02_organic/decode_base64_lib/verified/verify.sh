#!/bin/bash
# Full verification driver: enumerates every valid cargo feature combination
# (mechanically, from Cargo.toml), builds the C reference `.so`, then runs
# `cargo check` + the whole differential test suite for each combination.
#
# Usage: ./verify.sh            (dev profile)
#        ./verify.sh --release  (also run the suite against an optimized .so)
set -u
cd "$(dirname "$0")" || exit 1
CARGO_FLAGS="--offline"
EXTRA="${1:-}"

# ---------------------------------------------------------------- C reference
echo "=== building the C reference shared library ==="
(mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libdriver.so || exit 1

# ------------------------------------------------------- feature combinations
mapfile -t COMBOS < <(python3 - <<'PY'
import re, itertools
t = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', t, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
feats = [f for f in feats if f != 'default']
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(','.join(c))
PY
)
echo "=== ${#COMBOS[@]} feature combination(s): ${COMBOS[*]:-<empty set only>} ==="

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo
  echo "############ features: $label ############"
  set -x
  timeout 600 cargo check $CARGO_FLAGS --no-default-features --features "$combo" --all-targets || rc=1
  timeout 600 cargo build $CARGO_FLAGS --no-default-features --features "$combo" $EXTRA || rc=1
  timeout 600 cargo test  $CARGO_FLAGS --no-default-features --features "$combo" $EXTRA \
       -- --test-threads=4 || rc=1
  set +x
  echo "=== symbol diff (C -> Rust) for features: $label ==="
  profile=debug; [ "$EXTRA" = "--release" ] && profile=release
  missing=$(comm -23 \
       <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
       <(nm -D --defined-only target/$profile/libdriver.so | awk '{print $3}' | sort -u))
  n_c=$(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u | wc -l)
  if [ "$n_c" -eq 0 ]; then echo "BUG: no symbols read from the C .so"; rc=1; fi
  if [ -n "$missing" ]; then
      echo "MISSING SYMBOLS IN RUST:"; echo "$missing"; rc=1
  else
      echo "symbol diff empty: all $n_c C symbol(s) are exported by the Rust .so"
  fi
done

echo
if [ $rc -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES (rc=$rc)"; fi
exit $rc
