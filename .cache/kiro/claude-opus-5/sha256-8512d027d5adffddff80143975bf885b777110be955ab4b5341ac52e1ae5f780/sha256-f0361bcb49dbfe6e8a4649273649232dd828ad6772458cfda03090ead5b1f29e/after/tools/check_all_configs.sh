#!/usr/bin/env bash
# Phase D: run the differential suite across every feature combination and
# both profiles. Feature list is extracted from Cargo.toml, not hardcoded.
set -uo pipefail
cd "$(dirname "$0")/../translation" || exit 1

echo "===== features declared in Cargo.toml ====="
FEATURES=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.S | re.M)
if not m:
    sys.exit(0)
names = [n for n in re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)]
print(' '.join(n for n in names if n != 'default'))
PY
)
if [ -z "${FEATURES// /}" ]; then
    echo "(none declared -> exactly one build configuration)"
    COMBOS=("__default__")
else
    echo "features: $FEATURES"
    # power set of declared features
    mapfile -t COMBOS < <(python3 - "$FEATURES" <<'PY'
import itertools, sys
f = sys.argv[1].split()
print("__default__")
for r in range(len(f) + 1):
    for c in itertools.combinations(f, r):
        print(','.join(c) if c else "__none__")
PY
    )
fi

FAIL=0
for profile in release debug; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      __default__) FEAT_ARGS=() ;;
      __none__)    FEAT_ARGS=(--no-default-features) ;;
      *)           FEAT_ARGS=(--no-default-features --features "$combo") ;;
    esac
    PROF_ARGS=(); [ "$profile" = release ] && PROF_ARGS=(--release)

    echo ""
    echo "===== profile=$profile combo=$combo ====="
    if ! timeout 600 cargo build "${PROF_ARGS[@]}" "${FEAT_ARGS[@]}" >/dev/null 2>&1; then
        echo "BUILD FAILED"; FAIL=1; continue
    fi
    # Symbol parity for this configuration.
    SO="target/$profile/libhalf2float_lib.so"
    if ! nm -D --defined-only "$SO" | grep -q ' T half2float$'; then
        echo "SYMBOL half2float MISSING from $SO"; FAIL=1
    else
        echo "symbol half2float: present in $SO"
    fi
    OUT=$(timeout 600 cargo test "${PROF_ARGS[@]}" "${FEAT_ARGS[@]}" 2>&1)
    echo "$OUT" | grep -E 'test result:' | sed 's/^/  /'
    if echo "$OUT" | grep -qE 'FAILED|error:|panicked'; then
        echo "  *** FAILURES ***"; echo "$OUT" | grep -E 'FAILED|panicked|error:' | head -5
        FAIL=1
    fi
  done
done

echo ""
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$FAIL"
