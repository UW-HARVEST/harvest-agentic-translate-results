#!/usr/bin/env bash
# Enumerate EVERY valid feature combination of this crate and run
# `cargo check` + `cargo test` for each one.
#
# The crate declares no [features] table at all (see SYMBOLS.md), so the
# power set of features is {∅} and the three cargo spellings below all select
# the same configuration. The enumeration is still done mechanically from
# Cargo.toml so that adding a feature later cannot silently skip a combination.
set -uo pipefail
cd "$(dirname "$0")/.."

FEATURES=$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

echo "== features declared in Cargo.toml: [${FEATURES}]"

# Build the power set of the declared features.
combos=("")
for f in $FEATURES; do
    new=()
    for c in "${combos[@]}"; do
        new+=("$c")
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    combos=("${new[@]}")
done

rc=0
run() {
    echo
    echo "---- $* ----"
    if ! "$@"; then
        echo "!!! FAILED: $*"
        rc=1
    fi
}

for c in "${combos[@]}"; do
    run cargo check --no-default-features --features "$c"
done
run cargo check
run cargo check --all-features
run cargo check --no-default-features

for c in "${combos[@]}"; do
    run cargo build --no-default-features --features "$c"
    run cargo test --no-default-features --features "$c" -- --test-threads=4
done
run cargo build
run cargo test --all-features -- --test-threads=4

echo
if [ $rc -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS OK (${#combos[@]} power-set entries + default/all-features spellings)"
else
    echo "FEATURE COMBINATION FAILURES -- see above"
fi
exit $rc
