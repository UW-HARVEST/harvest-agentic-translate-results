#!/bin/bash
# Probe the C driver with a list of inputs and print the resulting hex bytes.
# Usage: probe.sh <file-with-one-input-per-line-in-printf-escapes>
C=./c_src/build/driver
R=./target/release/driver
while IFS= read -r line; do
  [ -z "$line" ] && continue
  c=$(printf '%b' "$line" | $C)
  r=$(printf '%b' "$line" | $R)
  if [ "$c" = "$r" ]; then st="ok  "; else st="DIFF"; fi
  printf '%s  %-24s C=%s R=%s\n' "$st" "$line" "$c" "$r"
done < "$1"
