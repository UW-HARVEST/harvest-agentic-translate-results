#!/usr/bin/env bash
# Phase D driver.
#
#  1. rebuilds the C shared library (default flags) plus out-of-tree -O1/-O2/-O3/-Os
#     variants, so the translation is checked against several compilations of the
#     same C source rather than one;
#  2. enumerates every Cargo feature combination;
#  3. for each combination: cargo check, cargo BUILD (required -- `cargo test`
#     does NOT build a cdylib), nm -D symbol diff, then the full test suite in
#     both the dev and release profiles;
#  4. re-runs the suite against every C optimisation level.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== 1. building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so (cmake defaults): $C_SO"

# Out-of-tree variants. c_src is never modified.
declare -a ALT_SOS=()
for O in O1 O2 O3 Os; do
  rm -rf "/tmp/c2_$O"
  if cmake -S "$ROOT/c_src" -B "/tmp/c2_$O" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-$O" >/dev/null 2>&1 \
     && cmake --build "/tmp/c2_$O" >/dev/null 2>&1; then
    ALT_SOS+=("-$O:$(ls /tmp/c2_$O/*.so | head -1)")
    echo "C  .so (-$O): $(ls /tmp/c2_$O/*.so | head -1)"
  fi
done

echo
echo "=== 2. enumerating feature combinations from Cargo.toml ==="
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
N=${#FEATURES[@]}
echo "declared features: $N ${FEATURES[*]:-(none)}"

COMBOS=("--<default>" "--no-default-features")
if [ "$N" -gt 0 ]; then
  for ((mask = 1; mask < (1 << N); mask++)); do
    sel=""
    for ((b = 0; b < N; b++)); do
      if (( mask & (1 << b) )); then sel="${sel:+$sel,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("--features $sel" "--no-default-features --features $sel")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

R_SO_REL="$ROOT/translation/target/release/libgjk_cache_lib.so"
R_SO_DBG="$ROOT/translation/target/debug/libgjk_cache_lib.so"

echo
echo "=== 3. per-combination: check, build, symbol diff, test (dev + release) ==="
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "--<default>" ]; then flags=(); label="(default features)";
  else read -r -a flags <<< "$combo"; label="$combo"; fi

  echo
  echo "--- $label ---"
  if ! timeout 600 cargo check "${flags[@]}" >/tmp/c2check.log 2>&1; then
    echo "  cargo check FAILED"; tail -20 /tmp/c2check.log; FAIL=1; continue
  fi
  echo "  cargo check ok"

  for profile in dev release; do
    if [ "$profile" = release ]; then pf=(--release); so="$R_SO_REL"; else pf=(); so="$R_SO_DBG"; fi

    # `cargo test` never builds a cdylib -- build it explicitly or the tests
    # would load a stale .so (the harness also asserts freshness).
    if ! timeout 600 cargo build "${pf[@]}" "${flags[@]}" >/tmp/c2build.log 2>&1; then
      echo "  [$profile] cargo build FAILED"; tail -20 /tmp/c2build.log; FAIL=1; continue
    fi

    nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/c2_c.txt
    nm -D --defined-only "$so"   | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/c2_r.txt
    missing="$(comm -23 /tmp/c2_c.txt /tmp/c2_r.txt)"
    if [ -n "$missing" ]; then
      echo "  [$profile] SYMBOL PARITY FAILED, missing from the Rust .so:"
      echo "$missing" | sed 's/^/      /'; FAIL=1
    else
      echo "  [$profile] symbol parity ok ($(wc -l < /tmp/c2_c.txt) symbols, 0 missing)"
    fi

    if ! timeout 600 cargo test "${pf[@]}" "${flags[@]}" >/tmp/c2test.log 2>&1; then
      echo "  [$profile] cargo test FAILED"
      grep -E "^test .* FAILED|panicked|SIGSEGV" /tmp/c2test.log | head -20 | sed 's/^/      /'
      FAIL=1; continue
    fi
    total=$(grep -E "^test result:" /tmp/c2test.log | awk '{s+=$4} END {print s}')
    echo "  [$profile] cargo test ok ($total tests passed)"
  done
done

echo
echo "=== 4. re-running the suite against each C optimisation level ==="
timeout 600 cargo build --release >/dev/null 2>&1
for entry in "${ALT_SOS[@]}"; do
  lvl="${entry%%:*}"; so="${entry#*:}"
  if C2_C_SO="$so" timeout 600 cargo test --release >/tmp/c2alt.log 2>&1; then
    total=$(grep -E "^test result:" /tmp/c2alt.log | awk '{s+=$4} END {print s}')
    echo "  C $lvl: ok ($total tests passed)"
  else
    echo "  C $lvl: FAILED"
    grep -E "^test .* FAILED|panicked" /tmp/c2alt.log | head -10 | sed 's/^/      /'
    FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "=== ALL COMBINATIONS PASSED ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit "$FAIL"
