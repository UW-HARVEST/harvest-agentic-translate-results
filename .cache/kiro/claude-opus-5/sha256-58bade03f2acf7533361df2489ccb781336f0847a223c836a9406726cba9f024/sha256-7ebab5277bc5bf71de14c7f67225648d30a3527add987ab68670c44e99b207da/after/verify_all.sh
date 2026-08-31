#!/usr/bin/env bash
# Enumerate every valid feature combination declared in translation/Cargo.toml,
# then cargo check + cargo test each one against the C shared library.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT/translation" || exit 1

# --- enumerate features from [features] ---------------------------------------
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
echo "Declared non-default features: $N ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "$N" -eq 0 ]; then
  COMBOS=("")            # only the empty combination exists
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Feature combinations to verify: ${#COMBOS[@]}"

# --- C shared library ---------------------------------------------------------
CSO="$ROOT/c_src/build/libdriver.so"
if [ ! -f "$CSO" ]; then
  (cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || exit 1
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo "=============================================================="
  echo "### combination: --no-default-features --features '$label'"
  echo "=============================================================="

  timeout 600 cargo check --no-default-features --features "$combo" 2>&1 | tail -5 || FAIL=1
  timeout 600 cargo build --release --no-default-features --features "$combo" 2>&1 | tail -3 || FAIL=1

  # symbol parity: every dynamic symbol the C .so defines must be defined by Rust
  c_syms=$(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "MISSING SYMBOLS in Rust .so:"
    echo "$missing"
    FAIL=1
  else
    echo "symbol parity OK ($(echo "$c_syms" | wc -l) C symbol(s) all present)"
  fi

  # differential test against the release cdylib and the debug cdylib
  for so in "$ROOT/translation/target/release/libdriver.so" ""; do
    if [ -n "$so" ]; then
      DRIVER_RUST_SO="$so" timeout 600 cargo test --no-default-features \
        --features "$combo" -- --test-threads=1 2>&1 | tail -8 || FAIL=1
    else
      timeout 600 cargo test --no-default-features --features "$combo" \
        -- --test-threads=1 2>&1 | tail -8 || FAIL=1
    fi
  done
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES DETECTED"; fi
exit "$FAIL"
