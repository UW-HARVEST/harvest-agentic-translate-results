#!/usr/bin/env bash
# Full differential verification: build both libraries, enumerate every cargo
# feature combination, and run the whole test suite under each one.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
ROOT=$PWD
RC=0

echo "=== 1. build the C shared library ==="
( cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/cbuild.log 2>&1 \
  || { echo "C BUILD FAILED"; tail -30 /tmp/cbuild.log; exit 1; }
ls -l c_src/build/libdriver.so

echo
echo "=== 2. enumerate cargo features ==="
# Features are read out of Cargo.toml rather than hard-coded.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,"");gsub(/ /,"");if($0!="default"&&$0!="")print}' \
    translation/Cargo.toml
)
echo "features found: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of configurations to test: the default build plus, if any
# features exist, every subset of them.
CONFIGS=("default")
if ((${#FEATURES[@]} > 0)); then
  CONFIGS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    CONFIGS+=("--no-default-features --features $combo")
  done
fi
echo "configurations to verify: ${#CONFIGS[@]}"

echo
for cfg in "${CONFIGS[@]}"; do
  if [[ $cfg == default ]]; then flags=(); else read -r -a flags <<<"$cfg"; fi
  echo "=== 3. configuration: $cfg ==="

  ( cd "$ROOT/translation" && timeout 600 cargo build --release "${flags[@]}" ) \
    >/tmp/rsbuild.log 2>&1 \
    || { echo "  RUST BUILD FAILED"; tail -30 /tmp/rsbuild.log; RC=1; continue; }

  echo "  --- nm -D symbol diff (C -> Rust) ---"
  nm -D --defined-only "$ROOT/c_src/build/libdriver.so" \
    | awk '$2=="T"{print $3}' | sort >/tmp/c_syms.txt
  nm -D --defined-only "$ROOT/translation/target/release/libdriver.so" \
    | awk '$2=="T"{print $3}' | sort >/tmp/rs_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)
  if [[ -n $missing ]]; then
    echo "  MISSING FROM RUST .so:"; echo "$missing" | sed 's/^/    /'; RC=1
  else
    echo "  symbol diff EMPTY ($(wc -l </tmp/c_syms.txt) symbols)"
  fi

  echo "  --- cargo test ---"
  ( cd "$ROOT/translation" && timeout 600 cargo test --release "${flags[@]}" ) \
    >/tmp/rstest.log 2>&1
  status=$?
  grep -E '^test result:' /tmp/rstest.log | sed 's/^/    /'
  if ((status != 0)); then
    echo "  TESTS FAILED (exit $status)"
    grep -E 'panicked|FAILED|DIVERGENCE' /tmp/rstest.log | head -20 | sed 's/^/    /'
    RC=1
  fi
  echo
done

if ((RC == 0)); then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $RC
