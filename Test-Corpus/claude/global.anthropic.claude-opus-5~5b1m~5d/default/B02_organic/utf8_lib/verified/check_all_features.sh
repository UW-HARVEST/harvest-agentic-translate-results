#!/usr/bin/env bash
# Phase D: enumerate every feature combination declared in Cargo.toml and run
# the whole differential suite for each one, against BOTH the debug and the
# release cdylib.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 2

# --- enumerate the features actually declared in Cargo.toml ----------------
features="$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    print('', end='')
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'))
PY
)"

echo "declared features: '${features:-<none>}'"

combos=()
if [ -z "$features" ]; then
    # no [features] table: the only two configurations that exist
    combos+=("")
    combos+=("--no-default-features")
else
    # full power set of the declared features, with and without defaults
    read -r -a farr <<< "$features"
    count=${#farr[@]}
    total=$((1 << count))
    for ((mask = 0; mask < total; mask++)); do
        sel=""
        for ((i = 0; i < count; i++)); do
            if (( mask & (1 << i) )); then sel="$sel,${farr[i]}"; fi
        done
        sel="${sel#,}"
        combos+=("--no-default-features${sel:+ --features $sel}")
        combos+=("${sel:+--features $sel}")
    done
fi

rc=0
for combo in "${combos[@]}"; do
    # shellcheck disable=SC2086
    if ! ./run_tests.sh $combo; then
        echo "#### FAILED for configuration: '${combo:-<default>}'"
        rc=1
    else
        echo "#### PASSED for configuration: '${combo:-<default>}'"
    fi
    echo
done

if [ "$rc" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS: OK"; else echo "SOME FEATURE COMBINATIONS FAILED"; fi
exit "$rc"
