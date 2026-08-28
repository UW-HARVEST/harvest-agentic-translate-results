#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run cargo check + cargo
# test against the C shared library for each one.
set -uo pipefail

cd "$(dirname "$0")"

# --- enumerate features declared in Cargo.toml -------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "declared non-default features: ${FEATURES[*]:-<none>}"

# every subset of the declared features (plus the plain default build)
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

# --- build the C reference library ------------------------------------------
echo "=== building C reference shared library"
(
  cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1
) || { echo "C build FAILED (see /tmp/cmake.log)"; exit 1; }

rc=0
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<no features>"
    args=(--no-default-features)
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi

  echo "=== cargo check   [$label]"
  if ! timeout 600 cargo check --all-targets "${args[@]}" >"/tmp/check.log" 2>&1; then
    echo "CHECK FAILED [$label]"; tail -n 30 /tmp/check.log; rc=1; continue
  fi

  echo "=== cargo test    [$label]"
  if ! timeout 600 cargo test "${args[@]}" >"/tmp/test.log" 2>&1; then
    echo "TEST FAILED [$label]"; grep -E "^(test |error|thread |assertion)" /tmp/test.log | tail -n 40; rc=1; continue
  fi
  grep -E "^test result:" /tmp/test.log
done

# also verify the default feature set builds and tests clean
echo "=== cargo check   [default]"
timeout 600 cargo check --all-targets >/tmp/check.log 2>&1 || { echo "CHECK FAILED [default]"; tail -n 30 /tmp/check.log; rc=1; }
echo "=== cargo test    [default]"
if timeout 600 cargo test >/tmp/test.log 2>&1; then
  grep -E "^test result:" /tmp/test.log
else
  echo "TEST FAILED [default]"; grep -E "^(test |error|thread |assertion)" /tmp/test.log | tail -n 40; rc=1
fi

# --- symbol parity ----------------------------------------------------------
echo "=== symbol parity"
cargo build --release >/dev/null 2>&1
C_SO=$(ls ../c_src/build/*.so | head -n1)
R_SO=target/release/libarr_del_lib.so
nm -D --defined-only "$C_SO" | awk 'NF>=3 && $2!="U" {print $3}' | sort -u >/tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk 'NF>=3 && $2!="U" {print $3}' | sort -u >/tmp/rs_syms.txt
missing=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)
if [ -n "$missing" ]; then
  echo "MISSING EXPORTS in Rust .so:"; echo "$missing"; rc=1
else
  echo "all $(wc -l </tmp/c_syms.txt) C exports present in the Rust .so"
fi

[ "$rc" -eq 0 ] && echo "ALL CONFIGURATIONS PASS" || echo "FAILURES PRESENT"
exit "$rc"
