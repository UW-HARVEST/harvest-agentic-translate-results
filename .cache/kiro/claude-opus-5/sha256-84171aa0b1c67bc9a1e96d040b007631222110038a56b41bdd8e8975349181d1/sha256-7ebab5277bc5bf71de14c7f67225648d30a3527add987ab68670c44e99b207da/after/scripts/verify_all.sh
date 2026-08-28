#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every declared
# feature combination: cargo check, cargo test (debug + release) and exported
# symbol parity.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$root/translation" || exit 1

# --- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#features[@]}
combos=()
if (( n == 0 )); then
  combos=("")            # only the featureless configuration exists
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    set=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then set="${set:+$set,}${features[$i]}"; fi
    done
    combos+=("$set")
  done
fi

echo "feature combinations to verify: ${#combos[@]}"
for c in "${combos[@]}"; do echo "  - [${c:-<none>}]"; done

# --- C reference ------------------------------------------------------------
if [[ ! -d "$root/c_src/build" ]]; then
  ( cd "$root/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || exit 1
fi
c_so="$(ls "$root"/c_src/build/*.so | head -1)"
c_syms="$(nm -D --defined-only "$c_so" | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u)"

rc=0
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  echo
  echo "=============== features: $label ==============="

  for step in "check" "test" "test --release" "build --release"; do
    # shellcheck disable=SC2086
    if timeout 600 cargo $step --no-default-features \
         ${combo:+--features "$combo"} > /tmp/verify.log 2>&1; then
      echo "  cargo $step: OK"
    else
      echo "  cargo $step: FAILED"
      tail -30 /tmp/verify.log
      rc=1
    fi
  done

  rust_so="target/release/libencode_quant_lib.so"
  if [[ -f $rust_so ]]; then
    rust_syms="$(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u)"
    missing="$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))"
    if [[ -n $missing ]]; then
      echo "  MISSING EXPORTS in Rust .so:"; echo "$missing" | sed 's/^/    /'
      rc=1
    else
      echo "  symbol parity: OK ($(echo "$c_syms" | wc -l) C symbol(s) all present)"
    fi
  else
    echo "  release .so not found"; rc=1
  fi
done

echo
if (( rc == 0 )); then echo "ALL FEATURE COMBINATIONS VERIFIED"; else echo "FAILURES PRESENT"; fi
exit $rc
