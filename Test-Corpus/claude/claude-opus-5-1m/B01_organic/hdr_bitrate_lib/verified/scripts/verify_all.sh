#!/usr/bin/env bash
# Full verification gate: Phase A artifacts, Phase D symbol parity,
# Phases B+C differential tests under every feature combination and profile.
set -uo pipefail
cd "$(dirname "$0")/.."
rc=0
hdr(){ echo; echo "=================================================================="; echo "$1"; echo "=================================================================="; }

hdr "Phase A: artifacts present"
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  if [ -s "$f" ]; then echo "  ok    $f ($(wc -l < "$f") lines)"; else echo "  MISSING $f"; rc=1; fi
done

hdr "Build C shared library (prescribed command)"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && echo "  ok    c_src/build/$(ls c_src/build/lib*.so | xargs -n1 basename)" || { echo "  FAIL  C build"; rc=1; }

hdr "Phase A/D: every feature combination compiles"
bash scripts/check_all_features.sh || rc=1

hdr "Phase D: symbol parity (nm -D)"
bash scripts/symbol_parity.sh || rc=1

hdr "Phases B+C: differential tests, all feature combos x both profiles"
bash scripts/run_all_features.sh || rc=1

hdr "RESULT"
[ "$rc" -eq 0 ] && echo "  ALL VERIFICATION GATES PASS" || echo "  VERIFICATION FAILED"
exit $rc
