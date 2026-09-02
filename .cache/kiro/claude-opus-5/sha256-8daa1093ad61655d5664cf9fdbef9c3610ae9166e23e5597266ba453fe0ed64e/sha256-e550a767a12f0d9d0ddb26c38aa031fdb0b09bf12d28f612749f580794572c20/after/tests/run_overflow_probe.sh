#!/bin/bash
# Robustness probe: build with overflow checks + debug assertions enabled and
# run the differential test.  Any panic indicates a place where the translation
# relies on wrapping arithmetic or on x86 shift masking without saying so.
set -u
ROOT=$HARVEST_WORKDIR
cd "$ROOT/translation"
RUSTFLAGS="-C overflow-checks=on -C debug-assertions=on" \
    cargo build --release --target-dir /tmp/ovf_target 2>&1 | grep -E "^error" -A 8 && exit 1
cd "$ROOT"
gcc -O1 -g -I c_src/include -o /tmp/jdiff/dt_ovf tests/difftest.c \
    -L /tmp/ovf_target/release -ljansson -Wl,-rpath,/tmp/ovf_target/release -lm || exit 1
cd /tmp/jdiff
./dt_ovf > ovf.out 2> ovf.err
rc=$?
echo "overflow-checked run exit=$rc"
if [ -s ovf.err ]; then
    echo "--- panic ---"
    head -6 ovf.err
else
    echo "no panics"
fi
if cmp -s c.out ovf.out; then
    echo "overflow-checked output IDENTICAL to C"
else
    echo "output differs from C at line $(diff c.out ovf.out | head -1)"
fi
