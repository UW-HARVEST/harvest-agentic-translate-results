#!/usr/bin/env bash
# Mutation check: verify the differential suite actually DETECTS divergence.
# Applies a series of small mutations to the Rust source, confirms the suite
# fails for each, then restores the original file.
set -u
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; rm -f "$ORIG"; }
trap restore EXIT

# description | sed expression
MUTATIONS=(
  "encode(): '+' -> '-'|s/b'+'/b'-'/"
  "encode(): boundary 26 -> 27|s/if u < 26 /if u < 27 /"
  "encode(): boundary 62 -> 61|s/if u == 62 /if u == 61 /"
  "capacity: +4 -> +3|s/wrapping_add(4)/wrapping_add(3)/"
  "capacity: \*4 -> \*3|s/wrapping_mul(4)/wrapping_mul(3)/"
  "padding: '=' -> '.'|s/b'='/b'.'/"
  "shift: b1 >> 2 -> b1 >> 1|s/let b4: u8 = b1 >> 2;/let b4: u8 = b1 >> 1;/"
  "mask: b3 \& 0x3f -> b3 \& 0x1f|s/b3 \& 0x3f/b3 \& 0x1f/"
  "loop stride: i += 3 -> i += 2|s/        i += 3;/        i += 2;/"
  "strlen mode: size == 0 -> size < 0|s/if size == 0 /if size < 0 /"
)

fail=0
for m in "${MUTATIONS[@]}"; do
  desc="${m%%|*}"; expr="${m#*|}"
  cp "$ORIG" src/lib.rs
  sed -i "$expr" src/lib.rs
  if diff -q "$ORIG" src/lib.rs >/dev/null; then
    echo "SKIP (mutation did not apply): $desc"
    fail=1
    continue
  fi
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    echo "SKIP (mutant does not compile): $desc"
    continue
  fi
  if timeout 300 cargo test --release >/dev/null 2>&1; then
    echo "NOT DETECTED: $desc   <-- suite is too weak"
    fail=1
  else
    echo "detected: $desc"
  fi
done

cp "$ORIG" src/lib.rs
timeout 300 cargo build --release >/dev/null 2>&1
if timeout 300 cargo test --release >/dev/null 2>&1; then
  echo "restored original: suite passes"
else
  echo "ERROR: suite fails on the restored original!"
  fail=1
fi
exit $fail
