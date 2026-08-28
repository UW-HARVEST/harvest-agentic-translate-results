#!/usr/bin/env bash
# Builds the C reference .so and the Rust cdylib, then runs the differential
# tests for every feature combination (this crate declares none, so the only
# combination is the empty one).
set -euo pipefail
crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(cd "$crate/.." && pwd)"

# --- C reference library -----------------------------------------------------
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
  && cmake --build . >>/tmp/cmake.log 2>&1)

# --- feature combinations ----------------------------------------------------
# Parsed from Cargo.toml [features]; empty means the single default build.
mapfile -t feats < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' \
    "$crate/Cargo.toml" | grep -v '^default$' || true
)

combos=("")
n=${#feats[@]}
if (( n > 0 )); then
  combos=()
  for ((m=0; m<(1<<n); m++)); do
    c=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && c="${c:+$c,}${feats[i]}"
    done
    combos+=("$c")
  done
fi

profile="${PROFILE:-release}"
flag=""; [[ "$profile" == release ]] && flag="--release"

for combo in "${combos[@]}"; do
  echo "=== features: '${combo:-<none>}' ==="
  fargs=(--no-default-features)
  [[ -n "$combo" ]] && fargs+=(--features "$combo")
  (( n == 0 )) && fargs=()

  cd "$crate"
  timeout 600 cargo check ${flag} "${fargs[@]}" 2>&1 | tail -5
  # the cdylib must exist on disk before the tests dlopen it
  timeout 600 cargo build ${flag} "${fargs[@]}" 2>&1 | tail -5
  timeout 600 cargo test  ${flag} "${fargs[@]}" 2>&1 | tail -25

  # --- exported symbol parity ------------------------------------------------
  cso=$(ls "$root"/c_src/build/lib*.so | head -1)
  rso="$crate/target/$profile/libread_side_info_lib.so"
  nm -D --defined-only "$cso" | awk '{print $2, $3}' | grep -E '^(T|W|B|D|R) ' \
    | awk '{print $2}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$rso" | awk '{print $3}' | sort -u > /tmp/rs_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt || true)
  if [[ -n "$missing" ]]; then
    echo "MISSING EXPORTS in Rust .so:"; echo "$missing"; exit 1
  fi
  echo "symbol parity OK ($(wc -l < /tmp/c_syms.txt) C exports all present)"
done
echo "ALL COMBINATIONS PASSED"
