#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared libraries and runs the
# differential test-suite for EVERY feature combination (Cargo.toml declares no
# [features], so the combination space is: {} == default).
set -uo pipefail
cd "$(dirname "$0")"
COMBOS="${TMPDIR:-/tmp}/.driver_combos"
export CARGO_NET_OFFLINE=true
fail=0

echo "### enumerating feature combinations from Cargo.toml"
COMBOS="$COMBOS" python3 - <<'PY'
import re, itertools, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            feats.append(line.split('=')[0].strip())
feats = [f for f in feats if f != 'default']
print("features found:", feats if feats else "(none)")
combos = []
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        combos.append(",".join(c))
import os
open(os.environ["COMBOS"], "w").write("\n".join(combos) + "\n")
print("combinations:", len(combos))
PY

echo "### building C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libdriver.so

while IFS= read -r combo; do
    if [ -z "$combo" ]; then
        args=(--no-default-features)
        label="<no features>"
    else
        args=(--no-default-features --features "$combo")
        label="$combo"
    fi
    echo
    echo "=============================================================="
    echo "### feature combination: $label"
    echo "=============================================================="
    timeout 600 cargo check  --offline "${args[@]}" 2>&1 | tail -3 || fail=1
    timeout 600 cargo build  --offline "${args[@]}" 2>&1 | tail -3 || fail=1

    echo "--- symbol parity (nm -D --defined-only) ---"
    diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
         <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort) \
         && echo "symbols identical" || { echo "SYMBOL DIFF"; fail=1; }

    echo "--- differential tests ---"
    timeout 600 cargo test --offline "${args[@]}" -- --test-threads=1 2>&1 | tail -5 || fail=1
done < "$COMBOS"

echo
echo "### default-features invocation (combination #2 of SYMBOLS.md)"
timeout 600 cargo check --offline 2>&1 | tail -2 || fail=1
timeout 600 cargo test  --offline -- --test-threads=1 2>&1 | tail -3 || fail=1

echo
echo "### parallel (default) test-thread run, harness robustness check"
timeout 600 cargo test --offline 2>&1 | tail -3 || fail=1

echo
echo "### release profile (panic=abort) Rust .so"
timeout 600 cargo build --offline --release 2>&1 | tail -2 || fail=1
diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort) \
     && echo "release symbols identical" || { echo "RELEASE SYMBOL DIFF"; fail=1; }
RUST_DRIVER_SO="$PWD/target/release/libdriver.so" \
    timeout 600 cargo test --offline -- --test-threads=1 2>&1 | tail -3 || fail=1

echo
if [ "$fail" = 0 ]; then echo "ALL VERIFICATION STEPS PASSED"; else echo "SOME STEPS FAILED"; fi
exit "$fail"
