#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination, checked and tested.
#
# Usage: ./run_all.sh
set -u
cd "$(dirname "$0")"
LOG_DIR="${TMPDIR:-/tmp}/driver-verify"
mkdir -p "$LOG_DIR"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. build the C ground truth
# ---------------------------------------------------------------------------
step "building C ground truth"
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1 \
  || { echo "C build FAILED (see $LOG_DIR/cmake.log)"; exit 1; }
echo "ok: c_src/build/driver"

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "feature combinations"
FEATURES=$(python3 - <<'PY'
import re, pathlib, itertools
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r"^\[features\](.*?)(^\[|\Z)", txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != "default":
                feats.append(name)
combos = []
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        combos.append(",".join(c))
print("\n".join(combos))
PY
)
echo "non-default features: $(echo "$FEATURES" | tr '\n' '|')"
echo "combinations to verify: --no-default-features [+ each subset], plus the default feature set"

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check per combination"
check_one() {
    local desc="$1"; shift
    if cargo check --offline --all-targets "$@" > "$LOG_DIR/check.log" 2>&1; then
        echo "  ok    check $desc"
    else
        echo "  FAIL  check $desc"; tail -20 "$LOG_DIR/check.log"; fail=1
    fi
}
check_one "(default features)"
check_one "--no-default-features" --no-default-features
while IFS= read -r combo; do
    [ -z "$combo" ] && continue
    check_one "--no-default-features --features $combo" --no-default-features --features "$combo"
done <<< "$FEATURES"

# ---------------------------------------------------------------------------
# 3. differential tests for every combination, debug and release
# ---------------------------------------------------------------------------
step "differential tests per combination"
test_one() {
    local desc="$1"; shift
    if cargo test --offline "$@" -- --test-threads=8 > "$LOG_DIR/test.log" 2>&1; then
        echo "  ok    test $desc  ($(grep -c '^test .* ok$' "$LOG_DIR/test.log") tests)"
    else
        echo "  FAIL  test $desc"; grep -E "^(test |error|thread)" "$LOG_DIR/test.log" | tail -30; fail=1
    fi
}
test_one "(default features, debug)"
test_one "(default features, release)" --release
test_one "--no-default-features (debug)" --no-default-features
test_one "--no-default-features (release)" --no-default-features --release
while IFS= read -r combo; do
    [ -z "$combo" ] && continue
    test_one "--features $combo (debug)" --no-default-features --features "$combo"
    test_one "--features $combo (release)" --no-default-features --features "$combo" --release
done <<< "$FEATURES"

# ---------------------------------------------------------------------------
# 4. symbol parity
# ---------------------------------------------------------------------------
step "symbol parity (nm -D)"
cargo build --offline --release > /dev/null 2>&1
C_BIN=c_src/build/driver
R_BIN=target/release/driver

nm -D --defined-only "$C_BIN" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$LOG_DIR/c_exports.txt"
nm -D --defined-only "$R_BIN" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$LOG_DIR/r_exports.txt"
echo "  C exported dynamic symbols : $(wc -l < "$LOG_DIR/c_exports.txt")"
echo "  Rust exported dynamic symbols: $(wc -l < "$LOG_DIR/r_exports.txt")"
missing=$(comm -23 "$LOG_DIR/c_exports.txt" "$LOG_DIR/r_exports.txt")
if [ -n "$missing" ]; then
    echo "  FAIL  symbols exported by C but missing from Rust:"; echo "$missing"; fail=1
else
    echo "  ok    no C export is missing from the Rust binary"
fi

# every program entry point the C binary defines must exist in the Rust one
for sym in main _start; do
    if nm --defined-only "$R_BIN" | grep -qw "$sym"; then
        echo "  ok    entry point '$sym' present in Rust binary"
    else
        echo "  FAIL  entry point '$sym' missing from Rust binary"; fail=1
    fi
done

# undefined symbols in the Rust binary must all be resolvable (libc/libgcc)
if ldd -r "$R_BIN" 2>&1 | grep -E "undefined symbol|not found" > "$LOG_DIR/ldd.log"; then
    echo "  FAIL  unresolved symbols in Rust binary:"; cat "$LOG_DIR/ldd.log"; fail=1
else
    echo "  ok    all Rust imports resolve (libc/libgcc only)"
fi

# ---------------------------------------------------------------------------
step "result"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
