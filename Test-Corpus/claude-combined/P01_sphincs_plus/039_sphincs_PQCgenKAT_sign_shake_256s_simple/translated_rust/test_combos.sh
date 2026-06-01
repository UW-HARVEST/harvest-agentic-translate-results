#!/bin/bash
set -e
BACKENDS="haraka sha2 shake blake"
THASHES="robust simple"
SECPARS="128s 128f 192s 192f 256s 256f"

cd "$(dirname "$0")"
for b in $BACKENDS; do
  for t in $THASHES; do
    for sp in $SECPARS; do
      echo "=== Checking $b/$t/$sp ==="
      cargo check --quiet --no-default-features --features "$b,$t,$sp" 2>&1 | grep -E "(error|warning: unused)" | head -3 || true
    done
  done
done
