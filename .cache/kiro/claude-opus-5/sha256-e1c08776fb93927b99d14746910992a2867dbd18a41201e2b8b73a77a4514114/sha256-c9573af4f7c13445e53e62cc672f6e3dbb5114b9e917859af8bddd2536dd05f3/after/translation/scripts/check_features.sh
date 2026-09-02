#!/usr/bin/env bash
# Phase D — run the whole verification for EVERY feature combination.
#
# Feature combinations are enumerated mechanically from Cargo.toml (powerset of
# the declared features, plus the --no-default-features baseline), so a feature
# added later is picked up without editing this script.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

mapfile -t combos < <(python3 - <<'PY'
import itertools, re, sys
text = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        name = line.split('=')[0].strip()
        if name and name != 'default':
            feats.append(name)
print("__default__")
print("__none__")
for n in range(1, len(feats) + 1):
    for combo in itertools.combinations(feats, n):
        print(",".join(combo))
PY
)

echo "feature combinations to verify: ${#combos[@]}"
printf '  %s\n' "${combos[@]}"

status=0
for combo in "${combos[@]}"; do
    case "$combo" in
        __default__) args=() ; label="default" ;;
        __none__)    args=(--no-default-features) ; label="no-default-features" ;;
        *)           args=(--no-default-features --features "$combo") ; label="features=$combo" ;;
    esac
    echo
    echo "=============================================================="
    echo "== $label"
    echo "=============================================================="
    for step in "build --release" "build" "test"; do
        # shellcheck disable=SC2086
        if ! timeout 600 cargo $step "${args[@]+"${args[@]}"}" >"/tmp/fc-$$.log" 2>&1; then
            echo "FAIL: cargo $step (${label})" >&2
            tail -n 30 "/tmp/fc-$$.log" >&2
            status=1
            continue
        fi
        echo "OK: cargo $step"
        if [[ "$step" == "test" ]]; then
            grep -E '^test result:' "/tmp/fc-$$.log" | sed 's/^/    /'
        fi
    done
    if ! ./scripts/symbol_parity.sh >"/tmp/fc-sym-$$.log" 2>&1; then
        echo "FAIL: symbol parity (${label})" >&2
        cat "/tmp/fc-sym-$$.log" >&2
        status=1
    else
        echo "OK: symbol parity"
    fi
done

rm -f "/tmp/fc-$$.log" "/tmp/fc-sym-$$.log"
echo
[[ $status -eq 0 ]] && echo "ALL FEATURE COMBINATIONS PASSED" || echo "FAILURES PRESENT" >&2
exit $status
