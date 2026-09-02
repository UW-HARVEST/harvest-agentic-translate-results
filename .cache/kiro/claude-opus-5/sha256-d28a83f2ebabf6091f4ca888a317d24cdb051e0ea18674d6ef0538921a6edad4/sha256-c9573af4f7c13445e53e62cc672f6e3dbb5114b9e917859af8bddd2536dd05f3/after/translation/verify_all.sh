#!/usr/bin/env bash
# Run the full differential verification across every cargo feature
# combination and both build profiles. Nothing here is hand-repeated.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
FAIL=0

# --- 1. build the C ground truth ------------------------------------------
echo "== building C .so =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# --- 2. enumerate features declared in Cargo.toml -------------------------
# Everything under [features], minus the implicit "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "== declared features: ${#FEATURES[@]} (${FEATURES[*]:-none}) =="

# Build the list of feature-flag argument sets to test: default, then
# --no-default-features alone, then every subset of the declared features.
COMBOS=("")                                  # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi

echo "== feature combinations to verify: ${#COMBOS[@]} =="

# --- 3. cargo check every combination -------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  if timeout 600 cargo check $combo >/tmp/chk.log 2>&1; then
    echo "  check  OK   : $label"
  else
    echo "  check  FAIL : $label"; tail -20 /tmp/chk.log; FAIL=1
  fi
done

# --- 4. build + test every combination in both profiles -------------------
for combo in "${COMBOS[@]}"; do
  for profile in release dev; do
    if [ "$profile" = release ]; then pflag="--release"; dir=release; else pflag=""; dir=debug; fi
    label="${combo:-<default>} [$profile]"

    if ! timeout 600 cargo build $pflag $combo >/tmp/bld.log 2>&1; then
      echo "  build  FAIL : $label"; tail -20 /tmp/bld.log; FAIL=1; continue
    fi
    so="$ROOT/translation/target/$dir/libdriver.so"
    if [ ! -f "$so" ]; then
      echo "  build  FAIL : $label (no .so at $so)"; FAIL=1; continue
    fi

    # symbol parity, checked directly in the shell as well as in the tests
    cdef=$(nm -D --defined-only --format=posix "$ROOT/c_src/build/libdriver.so" \
             | awk '$2=="T"||$2=="D"||$2=="B"||$2=="W"{print $1}' | sort)
    rdef=$(nm -D --defined-only --format=posix "$so" \
             | awk '$2=="T"||$2=="D"||$2=="B"||$2=="W"{print $1}' | sort)
    miss=$(comm -23 <(echo "$cdef") <(echo "$rdef") | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$')
    if [ -n "$miss" ]; then
      echo "  symbol FAIL : $label -> missing: $(echo $miss)"; FAIL=1
    fi

    if DRIVER_RUST_SO="$so" timeout 600 cargo test $pflag $combo -- --test-threads=1 \
         >/tmp/tst.log 2>&1; then
      passed=$(grep -c '^test .* ok$' /tmp/tst.log)
      echo "  test   OK   : $label ($passed tests)"
    else
      echo "  test   FAIL : $label"; grep -E 'FAILED|panicked|^test .* FAILED' /tmp/tst.log | head -20; FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $FAIL
