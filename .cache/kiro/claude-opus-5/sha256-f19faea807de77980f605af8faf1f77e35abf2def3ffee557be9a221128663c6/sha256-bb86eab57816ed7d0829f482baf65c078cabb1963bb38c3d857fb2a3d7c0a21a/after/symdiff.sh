#!/bin/bash
# symdiff.sh <backend> <thash> <secpar>  -- compare C .so symbol union vs Rust .so
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
B=$1; T=$2; S=$3
CD="$ROOT/c_build/${B}_${T}_${S}"
nm -D --defined-only "$CD/app/libsphincs_core_det.so" "$CD/app/libsphincs_core.so" "$CD/lib/$B/lib$B.so" \
  | awk 'NF==3{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$ROOT/translation/target/release/libsphincsplus.so" \
  | awk 'NF==3{print $3}' | sort -u > /tmp/rs_syms.txt
echo "C=$(wc -l </tmp/c_syms.txt) RS=$(wc -l </tmp/rs_syms.txt)"
echo "MISSING_IN_RUST:"; comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt | sed 's/^/  /'
echo "EXTRA_IN_RUST:"; comm -13 /tmp/c_syms.txt /tmp/rs_syms.txt | sed 's/^/  /'
echo "RUST_UNDEFINED_NONLIBC:"
nm -D -u "$ROOT/translation/target/release/libsphincsplus.so" | awk '{print $2}' \
  | grep -v '@GLIBC' | grep -v '^_ITM' | grep -v '^__' | sed 's/^/  /'
