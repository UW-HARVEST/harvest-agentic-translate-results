#!/usr/bin/env bash
# Full verification driver.
#
#   1. builds the C shared library
#   2. enumerates every Cargo feature combination declared in Cargo.toml
#   3. for each combination: cargo check + the whole differential suite,
#      in BOTH the debug and release profiles
#
# The Rust cdylib that the tests dlopen is rebuilt by the test harness itself
# (tests/common/mod.rs), using SO_UNDER_TEST_CARGO_ARGS so that the .so really
# is built with the same feature selection as the test binary.
set -u
cd "$(dirname "$0")"
ROOT=$(pwd)
TMP="$ROOT/.verif-tmp"
rm -rf "$TMP"; mkdir -p "$TMP"

fail=0

echo "==================================================================="
echo "0. Build the C shared library"
echo "==================================================================="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l ../c_src/build/libdriver.so

echo
echo "==================================================================="
echo "1. Enumerate feature combinations from Cargo.toml"
echo "==================================================================="
FEATURES=$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
# crude but sufficient: find the [features] table and list its keys
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        k = line.split('=')[0].strip().strip('"')
        if k and k != 'default':
            feats.append(k)
print(' '.join(feats))
PY
)
if [ -z "$FEATURES" ]; then
    echo "No [features] table in Cargo.toml -> the only configuration is the"
    echo "default (empty) feature set. Also exercising --no-default-features"
    echo "and --all-features for completeness."
    COMBOS=("" "--no-default-features" "--all-features")
else
    echo "declared features: $FEATURES"
    COMBOS=("" "--no-default-features" "--all-features")
    # every non-empty subset of the declared features, with defaults off
    python3 - "$FEATURES" > "$TMP/combos" <<'PY'
import itertools, sys
feats = sys.argv[1].split()
for r in range(1, len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print("--no-default-features --features " + ",".join(c))
PY
    while IFS= read -r line; do COMBOS+=("$line"); done < "$TMP/combos"
    rm -f "$TMP/combos"
fi
echo "combinations to test: ${#COMBOS[@]}"

echo
echo "==================================================================="
echo "2. cargo check + full differential suite per combination x profile"
echo "==================================================================="
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    relflag=""
    [ "$profile" = release ] && relflag="--release"
    label="profile=$profile features=[${combo:-<default>}]"

    if ! timeout 600 cargo check $relflag $combo --all-targets >"$TMP/chk" 2>&1; then
        echo "  CHECK FAIL  $label"
        tail -20 "$TMP/chk"
        fail=1; continue
    fi
    warns=$(grep -c "^warning" "$TMP/chk" || true)

    # Make the harness build the .so with the same features.
    export SO_UNDER_TEST_CARGO_ARGS="$combo"
    if timeout 600 cargo test $relflag $combo --tests >"$TMP/test" 2>&1; then
        n=$(grep -hoE "test result: ok\. [0-9]+ passed" "$TMP/test" \
            | grep -oE "[0-9]+" | awk '{t+=$1} END {print t+0}')
        echo "  PASS  $label   (${n:-?} tests, $warns warnings)"
    else
        echo "  FAIL  $label"
        grep -E "^test .*FAILED|panicked at|SIGSEGV|SIGABRT|^error" "$TMP/test" | head -15
        fail=1
    fi
    unset SO_UNDER_TEST_CARGO_ARGS
  done
done
rm -f "$TMP/chk" "$TMP/test"

echo
echo "==================================================================="
echo "3. Symbol diff (must be empty)"
echo "==================================================================="
cd "$ROOT"
cargo build --release >/dev/null 2>&1
nm -D --defined-only --format=posix ../c_src/build/libdriver.so \
  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $1}' | sort > "$TMP/csyms"
nm -D --defined-only --format=posix target/release/libdriver.so \
  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"||$2=="W"{print $1}' | sort > "$TMP/rsyms"
echo "C exports:"; cat "$TMP/csyms"
echo "Rust exports (filtered to C-API-shaped names):"
grep -vE '^(_init|_fini|_edata|_end|__bss_start|__data_start|data_start|__dso_handle|__TMC_END__|_ITM_|__gmon_start__|_ZN|_R|__rust|__rdl|__rg_|rust_)' "$TMP/rsyms" | head -20
if [ ! -s "$TMP/csyms" ]; then
    echo "FATAL: C symbol list is empty -- nm failed"; exit 1
fi
if [ ! -s "$TMP/rsyms" ]; then
    echo "FATAL: Rust symbol list is empty -- nm failed"; exit 1
fi
missing=$(comm -23 "$TMP/csyms" "$TMP/rsyms")
if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
else
    echo "symbol diff: EMPTY (every C export is present in the Rust .so)"
fi

echo
echo "==================================================================="
if [ $fail -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
echo "==================================================================="
exit $fail
