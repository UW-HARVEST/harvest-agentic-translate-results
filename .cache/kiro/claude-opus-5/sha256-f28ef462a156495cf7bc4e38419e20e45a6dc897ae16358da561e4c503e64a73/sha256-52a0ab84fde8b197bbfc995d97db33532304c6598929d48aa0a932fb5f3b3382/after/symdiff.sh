#!/bin/bash
# Compare `nm -D --defined-only` between the union of the three C shared
# libraries and the single Rust cdylib, for every configuration.
set -u
tot=0; bad=0
for d in /tmp/dif/*/; do
  combo=$(basename "$d")
  cat <(nm -D --defined-only "$d/libc_core_det.so") \
      <(nm -D --defined-only "$d/libc_core.so") \
      <(nm -D --defined-only "$d/libc_backend.so") \
      | awk '$2 ~ /^[TDBRWG]$/ {print $3}' | sort -u > /tmp/cs.txt
  nm -D --defined-only "$d/librs.so" \
      | awk '$2 ~ /^[TDBRWG]$/ {print $3}' \
      | grep -vE '^(_|rust_)' | sort -u > /tmp/rs.txt
  miss=$(comm -23 /tmp/cs.txt /tmp/rs.txt | tr '\n' ' ')
  extra=$(comm -13 /tmp/cs.txt /tmp/rs.txt | tr '\n' ' ')
  tot=$((tot+1))
  if [ -n "$miss" ]; then
    echo "$combo: MISSING: $miss"; bad=$((bad+1))
  fi
  if [ -n "$extra" ] && [ "${SHOW_EXTRA:-0}" = 1 ]; then
    echo "$combo: extra-in-rust: $extra"
  fi
done
echo "checked $tot combos, $bad with missing symbols"
exit $bad
