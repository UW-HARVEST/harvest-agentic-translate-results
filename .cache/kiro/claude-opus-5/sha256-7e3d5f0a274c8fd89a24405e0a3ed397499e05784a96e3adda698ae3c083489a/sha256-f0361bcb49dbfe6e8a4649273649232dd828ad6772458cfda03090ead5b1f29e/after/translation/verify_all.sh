#!/usr/bin/env bash
# Phase D driver: enumerate every cargo feature combination declared in
# Cargo.toml and run the full differential suite under each one.
#
# Usage:  ./verify_all.sh            (from translation/)
set -uo pipefail
cd "$(dirname "$0")"

C_SO=../c_src/build/libsodium.so
RS_SO=target/release/liblibsodium.so

# ---------------------------------------------------------------- build C
if [ ! -f "$C_SO" ]; then
  echo "== building the C shared library =="
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . -j"$(nproc)" >/dev/null 2>&1 )
fi
[ -f "$C_SO" ] || { echo "FAIL: C .so not built"; exit 1; }

# ------------------------------------------------- enumerate feature combos
# Extract the feature names declared under [features] (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== no [features] declared: the crate has exactly ONE configuration =="
  COMBOS+=("DEFAULT")
else
  # default build, no-default-features, each single feature, and all features
  COMBOS+=("DEFAULT" "--no-default-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--all-features")
fi

# ------------------------------------------------------------------- run
RC=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "DEFAULT" ]; then FLAGS=(); else read -r -a FLAGS <<< "$combo"; fi
  echo
  echo "==================== configuration: $combo ===================="

  echo "-- cargo build --release ${FLAGS[*]}"
  if ! timeout 600 cargo build --release "${FLAGS[@]}" >/tmp/build.log 2>&1; then
    echo "FAIL: build failed for [$combo]"; tail -20 /tmp/build.log; RC=1; continue
  fi

  # symbol parity for THIS configuration
  nm -D --defined-only "$C_SO"  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/rs_syms.txt
  MISSING=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)
  NMISS=$(printf '%s' "$MISSING" | grep -c . || true)
  echo "-- symbols: C=$(wc -l < /tmp/c_syms.txt) Rust=$(wc -l < /tmp/rs_syms.txt) missing=$NMISS"
  if [ "$NMISS" -ne 0 ]; then
    echo "FAIL: symbols missing from the Rust .so for [$combo]:"; echo "$MISSING"; RC=1
  fi

  for t in b1_sodium_core b2_stream b2_aead_box b3_sign_kem b4_hash_kdf_pwhash \
           c1_errors_core c2_errors_crypto c3_boundaries c4_misuse; do
    printf -- "-- %-22s " "$t"
    if timeout 900 cargo test --release "${FLAGS[@]}" --test "$t" >/tmp/t.log 2>&1; then
      grep -h '^test result:' /tmp/t.log | tr '\n' ' '; echo
    else
      echo "FAIL"; grep -E 'panicked|differs|FAILED|SIGSEGV|test result' /tmp/t.log | head -20; RC=1
    fi
  done
done

echo
if [ "$RC" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$RC"
