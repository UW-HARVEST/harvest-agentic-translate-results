#!/usr/bin/env bash
# Build both shared objects and run the differential rows for every feature
# combination. `cargo test` alone does NOT rebuild a cdylib, so the build step
# is mandatory before every test run.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# --- C shared library -------------------------------------------------------
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null )

# --- feature combinations ---------------------------------------------------
# Enumerated from Cargo.toml rather than hardcoded.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml
)

combos=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    combos+=("--no-default-features $( [ ${#sel[@]} -gt 0 ] && echo "--features $(IFS=,; echo "${sel[*]}")" )")
  done
else
  combos+=("--no-default-features")
  combos+=("--all-features")
fi

status=0
for combo in "${combos[@]}"; do
  flags=""
  [ "$combo" != "default" ] && flags="$combo"
  echo "==================================================================="
  echo "FEATURE COMBO: ${combo}"
  echo "==================================================================="
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $flags
  # shellcheck disable=SC2086
  timeout 600 cargo test --release $flags || status=1

  echo "--- nm -D symbol diff (C - Rust) ---"
  diff <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort) \
       > /tmp/symdiff.txt && echo "  symbol sets identical" || {
         if grep -q '^<' /tmp/symdiff.txt; then
           echo "  MISSING FROM RUST:"; grep '^<' /tmp/symdiff.txt; status=1
         fi
         grep '^>' /tmp/symdiff.txt | sed 's/^/  extra in Rust: /' || true
       }
  echo
done

exit $status
