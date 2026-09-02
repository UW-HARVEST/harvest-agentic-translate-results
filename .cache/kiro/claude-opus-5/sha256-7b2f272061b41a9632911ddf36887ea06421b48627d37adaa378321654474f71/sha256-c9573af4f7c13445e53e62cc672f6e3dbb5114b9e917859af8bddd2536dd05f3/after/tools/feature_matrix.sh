#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY feature combination.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are added later.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)
cd translation

FEATURES=$(python3 - <<'PY'
import re
s = open("Cargo.toml").read()
m = re.search(r"^\[features\]\s*$(.*?)(^\[|\Z)", s, re.S | re.M)
if not m:
    print("")
else:
    names = re.findall(r"^\s*([A-Za-z0-9_-]+)\s*=", m.group(1), re.M)
    print(" ".join(n for n in names if n != "default"))
PY
)

echo "features declared in Cargo.toml: '${FEATURES}'"

run_combo() {
    local label="$1"; shift
    echo
    echo "################ combination: ${label}  (cargo flags: $*) ################"
    if ! timeout 600 cargo build --release "$@" >/tmp/fc_build.log 2>&1; then
        echo "BUILD FAILED for ${label}"; tail -20 /tmp/fc_build.log; return 1
    fi
    nm -D --defined-only "$ROOT/c_src/build/libsodium.so" | awk '{print $3}' | sort -u > /tmp/fc_c.txt
    nm -D --defined-only target/release/liblibsodium.so    | awk '{print $3}' | sort -u > /tmp/fc_r.txt
    local missing
    missing=$(comm -23 /tmp/fc_c.txt /tmp/fc_r.txt | wc -l)
    echo "symbol parity: C=$(wc -l < /tmp/fc_c.txt) Rust=$(wc -l < /tmp/fc_r.txt) missing=${missing}"
    if [ "$missing" != "0" ]; then
        echo "MISSING SYMBOLS under ${label}:"; comm -23 /tmp/fc_c.txt /tmp/fc_r.txt; return 1
    fi
    if ! timeout 600 cargo test --release "$@" -- --test-threads=1 2>&1 | tee /tmp/fc_test.log | grep -E 'test result|FAILED'; then
        :
    fi
    if grep -q 'FAILED\|test result: FAILED' /tmp/fc_test.log; then
        echo "TESTS FAILED under ${label}"; return 1
    fi
    echo "combination ${label}: OK"
}

rc=0
if [ -z "${FEATURES}" ]; then
    echo
    echo "No [features] table: the crate has exactly ONE build configuration."
    echo "Running it three ways to prove --no-default-features / --all-features"
    echo "are equivalent to the default rather than assuming it."
    run_combo "default"              || rc=1
    run_combo "no-default-features"  --no-default-features || rc=1
    run_combo "all-features"         --all-features        || rc=1
else
    # power set of the declared features
    python3 - "$FEATURES" <<'PY' > /tmp/fc_combos.txt
import itertools, sys
feats = sys.argv[1].split()
print("")  # default
for n in range(len(feats) + 1):
    for c in itertools.combinations(feats, n):
        print(",".join(c))
PY
    while IFS= read -r combo; do
        if [ -z "$combo" ]; then
            run_combo "default" || rc=1
            run_combo "none" --no-default-features || rc=1
        else
            run_combo "$combo" --no-default-features --features "$combo" || rc=1
        fi
    done < /tmp/fc_combos.txt
fi

echo
if [ "$rc" = "0" ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
cargo build --release >/dev/null 2>&1
exit $rc
