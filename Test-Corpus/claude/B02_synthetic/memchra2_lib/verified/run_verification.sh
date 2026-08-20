#!/usr/bin/env bash
# Full verification run: every feature combination x (check, build, symbol
# parity, differential tests).
#
#   ./run_verification.sh
#
# Feature combinations are enumerated mechanically from Cargo.toml's [features]
# section (power set, `default` excluded since there is no default set).
set -u

cd "$(dirname "$0")" || exit 1

# --- 1. C shared library ---------------------------------------------------
if [ ! -f c_src/build/libtranslated_rust.so ]; then
    echo "### building C shared library"
    (mkdir -p c_src/build && cd c_src/build &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null) || exit 1
fi

# --- 2. enumerate feature combinations -------------------------------------
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys
text = open("Cargo.toml").read()
m = re.search(r"^\[features\](.*?)(^\[|\Z)", text, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name = line.split("=")[0].strip()
        if name and name != "default":
            feats.append(name)
for r in range(len(feats) + 1):
    for combo in itertools.combinations(feats, r):
        print(",".join(combo))
PY
)

echo "### feature combinations: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "    - '${c:-<none>}'"; done

FAIL=0
for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        ARGS=(--offline --no-default-features)
        label="<none>"
    else
        ARGS=(--offline --no-default-features --features "$combo")
        label="$combo"
    fi

    echo
    echo "############################################################"
    echo "### features: $label"
    echo "############################################################"

    echo "--- cargo check"
    cargo check "${ARGS[@]}" --all-targets 2>&1 | tail -n 5 || FAIL=1
    if ! cargo check "${ARGS[@]}" --all-targets >/dev/null 2>&1; then
        echo "CHECK FAILED for '$label'"; FAIL=1; continue
    fi

    echo "--- cargo build (cdylib)"
    if ! cargo build "${ARGS[@]}" >/dev/null 2>&1; then
        echo "BUILD FAILED for '$label'"; FAIL=1; continue
    fi

    echo "--- symbol parity"
    ./check_symbols.sh "$combo" | tail -n 8 || FAIL=1

    echo "--- differential tests"
    LOG="target/test-${label//[^A-Za-z0-9]/_}.log"
    if timeout 600 cargo test "${ARGS[@]}" > "$LOG" 2>&1; then
        grep -E 'Running|test result:' "$LOG"
    else
        grep -E 'Running|test result:|FAILED|panicked|left:|right:' "$LOG"
        echo "TESTS FAILED for '$label' (full log: $LOG)"
        FAIL=1
    fi
    grep -E '^(warning|error)' "$LOG" && { echo "WARNINGS for '$label'"; FAIL=1; }
done

# --- 3. extra configurations: release profile and an -O2 C build -----------
echo
echo "############################################################"
echo "### extra: release profile (panic = \"abort\", optimised)"
echo "############################################################"
cargo build --release --offline --no-default-features --features internal_test_api >/dev/null 2>&1 || FAIL=1
if timeout 600 cargo test --release --offline --no-default-features --features internal_test_api \
        > target/test-release.log 2>&1; then
    grep -E 'test result:' target/test-release.log
else
    grep -E 'test result:|FAILED|panicked' target/test-release.log; FAIL=1
fi

echo
echo "############################################################"
echo "### extra: C side compiled with -O2 (reference build is CMake's default)"
echo "############################################################"
cargo build --offline --no-default-features --features internal_test_api >/dev/null 2>&1 || FAIL=1
if CSHIM_CFLAGS=-O2 timeout 600 cargo test --offline --no-default-features \
        --features internal_test_api > target/test-o2.log 2>&1; then
    grep -E 'test result:' target/test-o2.log
else
    grep -E 'test result:|FAILED|panicked' target/test-o2.log; FAIL=1
fi

echo
if [ "$FAIL" -eq 0 ]; then
    echo "=== ALL FEATURE COMBINATIONS PASSED ==="
else
    echo "=== FAILURES PRESENT ==="
fi
exit "$FAIL"
