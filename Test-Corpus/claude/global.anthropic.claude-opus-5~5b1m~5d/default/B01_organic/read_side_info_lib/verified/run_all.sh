#!/usr/bin/env bash
# Phase D driver.
#
#   1. build the C shared library (both reference build types)
#   2. build the Rust shared library (every feature combo x both profiles)
#   3. diff the exported symbols (nm -D) -- must be empty
#   4. run the whole differential suite for every combination
#
# The C `.rodata` array order is optimisation dependent and is only observable
# through the out-of-range `sr_idx == 8` row, so each Rust feature combo is
# paired with the C build whose layout it reproduces:
#
#   default features  <->  cmake ..                              (-O0)
#   c_layout_o2       <->  cmake .. -DCMAKE_BUILD_TYPE=Release   (-O2)
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO="cargo --offline"
fail=0

hdr() { printf '\n==============================================================\n %s\n==============================================================\n' "$1"; }

hdr "1. Build the C shared library (both reference build types)"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C -O0 build FAILED"; exit 1; }
C_SO_O0=$(ls "$ROOT"/c_src/build/lib*.so | head -1)

mkdir -p build-c-o2
( cd build-c-o2 && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_BUILD_TYPE=Release >/dev/null && cmake --build . >/dev/null ) \
  || { echo "C -O2 build FAILED"; exit 1; }
C_SO_O2=$(ls "$PWD"/build-c-o2/lib*.so | head -1)
echo "C -O0 .so: $C_SO_O0"
echo "C -O2 .so: $C_SO_O2"

hdr "2. Feature combinations declared in Cargo.toml"
FEATS=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
out = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if line and not line.startswith('#') and '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                out.append(n)
print(' '.join(out))
PY
)
echo "non-default features: ${FEATS:-<none>}"
# Full power set of the declared features, each run with --no-default-features,
# plus the plain default build and --all-features.
COMBOS=("")                      # default features
COMBOS+=("--no-default-features")
if [ -n "$FEATS" ]; then
  set -- $FEATS
  n=$#; total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then eval "f=\${$((i + 1))}"; combo="$combo,$f"; fi
    done
    COMBOS+=("--no-default-features --features ${combo#,}")
  done
fi
COMBOS+=("--all-features")
printf 'combinations to verify (%d):\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  cargo test ${c:-<default>}"; done

syms() { nm -D --defined-only "$1" | awk '{print $NF}' | sort -u; }

hdr "3+4. Symbol parity and differential suite, per combination x profile"
for FLAGS in "${COMBOS[@]}"; do
  # Which C build does this Rust configuration reproduce?
  O2=no
  [[ "$FLAGS" == *c_layout_o2* ]] && O2=yes
  # --all-features turns on every declared feature, c_layout_o2 included.
  [[ "$FLAGS" == *--all-features* && "$FEATS" == *c_layout_o2* ]] && O2=yes
  if [ "$O2" = yes ]; then
    THIS_C="$C_SO_O2"; ROW8="long"
  else
    THIS_C="$C_SO_O0"; ROW8="mixed"
  fi
  for PROF in debug release; do
    LABEL="${FLAGS:-<default features>} / $PROF"
    echo
    echo ">>> $LABEL   (C: $(basename "$(dirname "$THIS_C")")/$(basename "$THIS_C"), row8=$ROW8)"

    if [ "$PROF" = release ]; then BF="--release"; else BF=""; fi
    # shellcheck disable=SC2086
    $CARGO build $BF $FLAGS -q || { echo "   BUILD FAILED"; fail=1; continue; }
    R_SO="target/$PROF/libread_side_info_lib.so"

    MISSING=$(comm -23 <(syms "$THIS_C") <(syms "$R_SO"))
    if [ -n "$MISSING" ]; then
      echo "   SYMBOL PARITY FAILED, missing from Rust:"; echo "$MISSING" | sed 's/^/     /'
      fail=1
    else
      echo "   symbols: OK ($(syms "$THIS_C" | wc -l) exported by C, 0 missing from Rust)"
    fi

    # shellcheck disable=SC2086
    OUT=$(C_SO="$THIS_C" RUST_SO_PROFILE="$PROF" UNREPRODUCIBLE_ROW8="$ROW8" \
          $CARGO test $FLAGS -- --test-threads=4 2>&1)
    echo "$OUT" | grep -E '^test result' | sed 's/^/   /'
    if echo "$OUT" | grep -q 'FAILED\|panicked'; then
      echo "$OUT" | grep -E '^test .*FAILED|DIVERGENCE|panicked' | head -20 | sed 's/^/   /'
      fail=1
    fi
  done
done

# Restore the default-feature debug/release .so so a bare `cargo test` works.
$CARGO build -q; $CARGO build --release -q

hdr "$([ "$fail" -eq 0 ] && echo 'ALL PHASES PASSED' || echo 'FAILURES DETECTED')"
exit "$fail"
