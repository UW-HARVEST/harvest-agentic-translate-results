#!/bin/bash
# Mechanically verify that every symbol exported by the C .so's is actually
# named by at least one differential test.  Usage: ./check_coverage.sh [backend]
R="$(cd "$(dirname "$0")" && pwd)"
b=${1:-blake}; s=${2:-128f}; t=${3:-simple}
CB="$R/cbuild/$b-$s-$t"

csyms=$( { nm -D --defined-only "$CB/app/libsphincs_core_det.so"; \
           nm -D --defined-only "$CB/app/libsphincs_core.so"; \
           nm -D --defined-only "$CB/lib/$b/lib$b.so"; } | awk '{print $3}' | grep -v '^$' | sort -u )

# Every symbol name literal that appears in the test sources (including the
# ones built by format! from a prefix).
tested=$(cat "$R"/translation/tests/*.rs "$R"/translation/tests/common/*.rs \
  | grep -oE '"[A-Za-z_][A-Za-z0-9_]*"' | tr -d '"' | sort -u)
prefixes=$(cat "$R"/translation/tests/*.rs | grep -oE '\{prefix\}_[a-z0-9_]+|\{variant\}_[a-z0-9_]+' \
  | sed 's/{prefix}_//; s/{variant}_//' | sort -u)

echo "### $b-$s-$t: C symbols never named by a test"
miss=0
for sym in $csyms; do
  if echo "$tested" | grep -qx "$sym"; then continue; fi
  hit=0
  for p in $prefixes; do
    case "$sym" in
      *"$p") hit=1; break;;
    esac
  done
  [ "$hit" = 1 ] && continue
  echo "  UNTESTED: $sym"
  miss=$((miss+1))
done
echo "  -> $miss untested of $(echo "$csyms" | wc -l)"
exit $((miss > 0))
