#!/bin/sh
# Enumerate every build-time configuration and check it.
#
# Cargo.toml has no [features] section and c_src/CMakeLists.txt has no options,
# so there is exactly one configuration; the loop below derives that from the
# manifest instead of assuming it.
set -e
cd "$(dirname "$0")/.."

features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml)
if [ -z "$features" ]; then
    echo "Cargo.toml declares no [features]; the only configuration is the default"
    combos=""
else
    echo "features: $features"
    combos=$(python3 - "$features" <<'PY'
import itertools, sys
feats = sys.argv[1].split()
for n in range(len(feats) + 1):
    for c in itertools.combinations(feats, n):
        print(",".join(c))
PY
)
fi

run() {
    echo "=== cargo $1 --no-default-features${2:+ --features $2} ==="
    if [ -n "$2" ]; then
        cargo "$1" --offline --no-default-features --features "$2" --all-targets
    else
        cargo "$1" --offline --no-default-features --all-targets
    fi
}

if [ -z "$combos" ]; then
    run check ""
else
    for c in $combos; do
        run check "$c"
    done
fi
echo "all configurations check out"
