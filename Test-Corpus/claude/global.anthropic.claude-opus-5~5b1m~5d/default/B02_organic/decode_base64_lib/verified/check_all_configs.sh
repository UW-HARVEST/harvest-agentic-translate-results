#!/usr/bin/env bash
# Phase D — run the whole differential suite under every build configuration.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are added later.
set -uo pipefail
cd "$(dirname "$0")"

# --- enumerate features declared in Cargo.toml -------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if not m:
    raise SystemExit
for line in m.group(1).splitlines():
    line = line.strip()
    if not line or line.startswith('#'):
        continue
    name = line.split('=')[0].strip()
    if name and name != 'default':
        print(name)
PY
)

# Build the combination list: default, no-default, then each feature and the
# full power set (capped to keep the run bounded).
COMBOS=("DEFAULT" "NO_DEFAULT")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
    total=$((1 << n))
    if [ "$total" -gt 64 ]; then total=64; fi
    for ((mask = 1; mask < total; mask++)); do
        sel=()
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then sel+=("${FEATURES[$i]}"); fi
        done
        COMBOS+=("$(
            IFS=,
            echo "${sel[*]}"
        )")
    done
fi

echo "features declared: ${n} ${FEATURES[*]:-(none)}"
echo "configurations to verify: ${#COMBOS[@]}"

rc=0
for profile in debug release; do
    prof_flag=""
    [ "$profile" = release ] && prof_flag="--release"
    for combo in "${COMBOS[@]}"; do
        case "$combo" in
        DEFAULT) feat_flags=() ;;
        NO_DEFAULT) feat_flags=(--no-default-features) ;;
        *) feat_flags=(--no-default-features --features "$combo") ;;
        esac
        label="profile=$profile features=$combo"

        # The cdylib must exist for the requested profile before the tests run,
        # because the tests dlopen it.
        if ! cargo build $prof_flag "${feat_flags[@]}" >/dev/null 2>&1; then
            echo "FAIL(build)  $label"
            rc=1
            continue
        fi
        out=$(timeout 600 cargo test $prof_flag "${feat_flags[@]}" 2>&1)
        if [ $? -ne 0 ]; then
            echo "FAIL(test)   $label"
            echo "$out" | tail -25
            rc=1
            continue
        fi
        bins=$(echo "$out" | grep -c '^test result: ok')
        ntests=$(echo "$out" | grep -oP '^test result: ok\. \K[0-9]+' |
            awk '{s += $1} END {print s + 0}')
        failed=$(echo "$out" | grep -c '^test result: FAILED')
        if [ "$failed" -ne 0 ] || [ "${ntests:-0}" -lt 40 ]; then
            echo "FAIL(sanity) $label  (only ${ntests:-0} tests ran, ${failed} failures)"
            echo "$out" | tail -25
            rc=1
        else
            echo "PASS         $label  (${ntests} tests across ${bins} binaries)"
        fi
    done
done

echo
if [ "$rc" -eq 0 ]; then
    echo "=== ALL CONFIGURATIONS PASS ==="
else
    echo "=== SOME CONFIGURATIONS FAILED ==="
fi
exit "$rc"
