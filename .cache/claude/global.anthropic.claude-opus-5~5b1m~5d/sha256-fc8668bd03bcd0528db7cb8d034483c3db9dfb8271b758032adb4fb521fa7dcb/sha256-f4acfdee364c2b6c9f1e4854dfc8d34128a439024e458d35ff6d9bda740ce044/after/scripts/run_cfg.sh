#!/bin/bash
# Build + differential-test ONE configuration in complete isolation
# (its own CARGO_TARGET_DIR so parallel runs and feature switches cannot race).
#
# Usage: run_cfg.sh <backend> <secpar> <thash> [extra cargo test args...]
set -u
b="$1"; s="$2"; t="$3"; shift 3
W="$(cd "$(dirname "$0")/.." && pwd)"
cfg="${b}_${s}_${t}"
log="$W/work/test_$cfg.log"
mkdir -p "$W/work" "$W/rustlibs"
export CARGO_TARGET_DIR="$W/tgt/$cfg"

{
  echo "=== $cfg ==="
  "$W/scripts/build_c.sh" "$b" "$s" "$t" || { echo "C-BUILD-FAIL"; exit 1; }
  cd "$W/translation" || exit 1
  cargo build --offline --release --quiet --no-default-features --features "$b,$t,$s" \
    || { echo "RUST-BUILD-FAIL"; exit 1; }
  cp "$CARGO_TARGET_DIR/release/lib005_sphincs_PQCgenKAT_sign_blake_128f_simple.so" \
     "$W/rustlibs/librust_$cfg.so"

  # symbol parity
  nm -D --defined-only "$W/cbuild/$cfg/libc_sphincs.so" | awk '{print $3}' | sort -u > "$W/work/c_$cfg.syms"
  nm -D --defined-only "$W/rustlibs/librust_$cfg.so"    | awk '{print $3}' | sort -u > "$W/work/r_$cfg.syms"
  miss=$(comm -23 "$W/work/c_$cfg.syms" "$W/work/r_$cfg.syms" | tr '\n' ' ')
  extra=$(comm -13 "$W/work/c_$cfg.syms" "$W/work/r_$cfg.syms" | tr '\n' ' ')
  if [ -n "$miss" ] || [ -n "$extra" ]; then
    echo "SYMDIFF missing=[$miss] extra=[$extra]"
    exit 1
  fi
  echo "SYMBOLS-OK $(wc -l < "$W/work/c_$cfg.syms")"

  # RUST_MIN_STACK: cargo's test threads default to a 2 MiB stack, which the
  # deep sign/verify call chains (C VLAs + large Rust stack buffers for the
  # 256-bit parameter sets) can exhaust. This is a harness limit, not a
  # translation issue.
  SPX_C_LIB="$W/cbuild/$cfg/libc_sphincs.so" \
  SPX_RUST_LIB="$W/rustlibs/librust_$cfg.so" \
  RUST_MIN_STACK=33554432 \
  cargo test --offline --release --no-default-features --features "$b,$t,$s" "$@"
} > "$log" 2>&1
rc=$?

if [ $rc -ne 0 ] || grep -qE 'FAILED|panicked|SYMDIFF|BUILD-FAIL|^error' "$log"; then
  echo "FAIL    $cfg   (see $log)"
  exit 1
fi
np=$(grep -oE '[0-9]+ passed' "$log" | awk '{s+=$1} END {print s}')
echo "ok      $cfg   ($np assertions-groups passed, $(grep -c 'test result: ok' "$log") binaries)"
