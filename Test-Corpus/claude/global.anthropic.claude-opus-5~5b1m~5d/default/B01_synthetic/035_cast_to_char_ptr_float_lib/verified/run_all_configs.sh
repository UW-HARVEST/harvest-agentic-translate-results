#!/usr/bin/env bash
# Runs the full differential suite (Phases B, C, D) across every feature
# combination declared in Cargo.toml and across both cargo profiles.
#
# Feature combos are extracted MECHANICALLY from Cargo.toml, so if a
# [features] table is ever added the matrix grows automatically.
set -uo pipefail
cd "$(dirname "$0")"

# --- build the C ground truth ---------------------------------------------
(cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }

# --- enumerate feature combinations --------------------------------------
FEATURES=$(python3 - <<'PY'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name not in ('default',):
                feats.append(name)
print(' '.join(feats))
PY
)

COMBOS=()
COMBOS+=("default:")                       # default feature set
COMBOS+=("no-default:--no-default-features")
if [ -n "$FEATURES" ]; then
  # power set of the declared features, each with --no-default-features
  mapfile -t EXTRA < <(python3 - "$FEATURES" <<'PY'
import itertools, sys
feats = sys.argv[1].split()
for r in range(1, len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print("%s:--no-default-features --features %s" % ('+'.join(c), ','.join(c)))
        print("%s+def:--features %s" % ('+'.join(c), ','.join(c)))
PY
)
  COMBOS+=("${EXTRA[@]}")
fi

fail=0
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="${combo%%:*}"
    flags="${combo#*:}"
    tag="profile=${profile_flag:-dev} features=${label}"
    echo "=============================================================="
    echo ">>> $tag"
    # cargo check first, then the differential suite. DIFFTEST_CARGO_ARGS makes
    # the harness build the cdylib under test with the same feature selection.
    if ! cargo check --offline --tests $profile_flag $flags >/dev/null 2>&1; then
      echo "!!! cargo check FAILED for $tag"; fail=1; continue
    fi
    DIFFTEST_CARGO_ARGS="$flags" \
      timeout 600 cargo test --offline $profile_flag $flags 2>&1 \
      | grep -E '^(test result|error|failures:$)|FAILED' \
      | sed 's/^/    /'
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
      echo "!!! TESTS FAILED for $tag"; fail=1
    else
      echo "    OK: $tag"
    fi
  done
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"
