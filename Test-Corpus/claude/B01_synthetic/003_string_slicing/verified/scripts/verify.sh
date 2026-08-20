#!/usr/bin/env bash
# Full verification matrix for the C -> Rust translation of c_src/src/main.c.
#
#   * Phase A : artifacts (SYMBOLS.md / ERRORS.md / CONFIGS.md) + symbol parity
#   * Phase B : tests/differential_so.rs  (CONFIGS.md rows C1-C20)
#              tests/differential_cli.rs (CONFIGS.md rows C21-C23)
#   * Phase C : tests/errors_so.rs        (ERRORS.md rows E1-E7, B1, B2, B6-B8, B10)
#              tests/crash_paths.rs      (ERRORS.md rows B3-B5)
#   * Phase D : tests/symbols.rs + every build configuration
#
# Everything runs with --test-threads=1 because the differential harness
# redirects file descriptor 1 while a library under test is running.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
CARGO_FLAGS=${CARGO_FLAGS:---offline}

echo "=============================================================="
echo "0. C artifacts"
echo "=============================================================="
# executable, exactly as c_src/CMakeLists.txt specifies (default flags == -O0)
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build c_src/build >/dev/null
# same source, optimized, in a build tree outside c_src/
mkdir -p target/cdiff
cmake -S c_src -B target/cdiff/cbuild-O2 \
      -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build target/cdiff/cbuild-O2 >/dev/null
# shared library (built by the test harness as well, done here for the symbol diff)
gcc -shared -fPIC -O2 -o target/cdiff/libcdriver.so c_src/src/main.c
ls -l c_src/build/driver target/cdiff/cbuild-O2/driver target/cdiff/libcdriver.so

echo
echo "=============================================================="
echo "1. Feature combinations (Cargo.toml declares no [features],"
echo "   so the complete set is the single default combination)"
echo "=============================================================="
FEATURE_COMBOS=$(python3 - <<'PY'
import re, sys
text = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            names.append(line.split('=')[0].strip())
names = [n for n in names if n != 'default']
combos = ['']
for i in range(1, 1 << len(names)):
    combos.append(','.join(n for j, n in enumerate(names) if i >> j & 1))
print('\n'.join(combos))
PY
)
while IFS= read -r combo; do
    echo "--- cargo check --no-default-features --features '${combo}'"
    cargo check $CARGO_FLAGS --no-default-features --features "$combo" --all-targets
done <<< "$FEATURE_COMBOS"

echo
echo "=============================================================="
echo "2. Symbol parity (nm -D)"
echo "=============================================================="
cargo build $CARGO_FLAGS >/dev/null
diff <(nm -D --defined-only target/cdiff/libcdriver.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort) \
  && echo "OK: the Rust .so exports every symbol the C .so exports"

echo
echo "=============================================================="
echo "3. Differential test matrix"
echo "=============================================================="
for profile in dev release; do
    if [ "$profile" = release ]; then
        BUILD_FLAG=--release
    else
        BUILD_FLAG=
    fi
    cargo build $CARGO_FLAGS $BUILD_FLAG >/dev/null
    for cflags in -O0 -O2; do
        if [ "$cflags" = "-O2" ]; then
            C_EXE=$ROOT/target/cdiff/cbuild-O2/driver
        else
            C_EXE=$ROOT/c_src/build/driver
        fi
        echo
        echo "### rust profile=$profile   C flags=$cflags"
        CDIFF_CFLAGS=$cflags CDIFF_C_EXE=$C_EXE \
            cargo test $CARGO_FLAGS $BUILD_FLAG -- --test-threads=1
    done
done

echo
echo "=============================================================="
echo "ALL CONFIGURATIONS PASSED"
echo "=============================================================="
