#!/bin/bash
# Per-configuration symbol parity check: every symbol exported by the C shared
# libraries (libsphincs_core_det.so + lib<backend>.so) must also be exported by
# the Rust cdylib.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"

total_missing=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      combo="$b-$t-$s"
      c="$ROOT/cbuild/$combo"
      r="$ROOT/rbuild/$combo/libsphincs_core_det.so"
      nm -D --defined-only "$c/libsphincs_core_det.so" "$c/lib$b.so" 2>/dev/null \
        | awk 'NF>=3{print $3}' | sort -u > ${TMPDIR:-/tmp}/claude_c_$$.txt
      nm -D --defined-only "$r" 2>/dev/null | awk 'NF>=3{print $3}' | sort -u > ${TMPDIR:-/tmp}/claude_r_$$.txt
      miss=$(comm -23 ${TMPDIR:-/tmp}/claude_c_$$.txt ${TMPDIR:-/tmp}/claude_r_$$.txt | tr '\n' ' ')
      if [ -n "${miss// /}" ]; then
        echo "MISSING $combo: $miss"
        total_missing=$((total_missing+1))
      else
        echo "ok      $combo (all $(wc -l < ${TMPDIR:-/tmp}/claude_c_$$.txt) C symbols present)"
      fi
      rm -f ${TMPDIR:-/tmp}/claude_c_$$.txt ${TMPDIR:-/tmp}/claude_r_$$.txt
    done
  done
done
echo "configs with missing symbols: $total_missing"
exit $((total_missing>0))
