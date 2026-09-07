#!/bin/bash
# Print the union of exported (dynamic, defined) symbols of the three C .so
# files for one configuration.
# Usage: c_syms.sh <backend> <thash> <secpar>
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
d="$ROOT/c_build/$1-$2-$3"
for f in "$d"/lib/$1/lib$1.so "$d"/app/libsphincs_core.so "$d"/app/libsphincs_core_det.so; do
  nm -D --defined-only "$f"
done | awk '$2 != "" {print $3}' | grep -v '^$' | sort -u
