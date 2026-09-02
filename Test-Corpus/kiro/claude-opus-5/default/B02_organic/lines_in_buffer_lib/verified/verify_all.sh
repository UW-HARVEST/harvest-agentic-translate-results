#!/bin/bash
# Runs the complete Phase A-D verification gate from scratch.
#   ./translation/verify_all.sh
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rc=0
step() { echo; echo "=== $* ==="; }

step "Phase A.1 - build the C shared library"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build "$ROOT/c_src/build" >/dev/null && echo OK || { echo FAIL; rc=1; }

step "Phase A.2 - cargo check (must be clean)"
(cd "$ROOT/translation" && timeout 600 cargo check 2>&1 | tail -3) || rc=1

step "Phase A.3 / D.1 - symbol parity (nm -D)"
cargo_so="$ROOT/translation/target/release/libdriver.so"
(cd "$ROOT/translation" && timeout 600 cargo build --release >/dev/null 2>&1)
c_syms=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u)
r_syms=$(nm -D --defined-only "$cargo_so" | awk '{print $NF}' | sort -u)
echo "C exports  : $(echo "$c_syms" | wc -l)"
echo "Rust exports: $(echo "$r_syms" | wc -l)"
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
if [ -n "$missing" ]; then echo "MISSING FROM RUST: $missing"; rc=1; else echo "0 missing - parity OK"; fi
if ldd -r "$cargo_so" 2>&1 | grep -qi undefined; then
  echo "FAIL: undefined symbols in Rust .so"; rc=1
else echo "0 undefined non-libc symbols"; fi

step "Phase B + C - differential suite (release .so)"
(cd "$ROOT/translation" && timeout 600 cargo test --release 2>&1 | grep -E '^test result:' | tail -1) || rc=1

step "Phase B + C - differential suite (debug .so, overflow checks on)"
(cd "$ROOT/translation" && timeout 600 cargo build >/dev/null 2>&1 \
  && RUST_DRIVER_SO="$ROOT/translation/target/debug/libdriver.so" \
     timeout 600 cargo test --release 2>&1 | grep -E '^test result:' | tail -1) || rc=1

step "Phase C strengthening - allocator interposer active"
(cd "$ROOT/translation" && timeout 600 ./run_with_interpose.sh 2>&1 | grep -E '^test result:|SKIP' | tail -2) || rc=1

step "Cross-check against the C at several optimisation levels"
for opt in -O0 -O2 -O3; do
  d="/tmp/cbuild_verify$opt"
  cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >/dev/null 2>&1 && cmake --build "$d" >/dev/null 2>&1 || continue
  echo "C $opt -> $(cd "$ROOT/translation" && C_DRIVER_SO="$d/libdriver.so" \
       timeout 600 cargo test --release 2>&1 | grep -E '^test result:' | tail -1)"
done

step "Phase D.2 - every feature combination"
(cd "$ROOT/translation" && timeout 900 ./phase_d_features.sh 2>&1 | tail -6) || rc=1

step "Harness adequacy - mutation sweep"
timeout 900 "$ROOT/translation/mutation_sweep.sh" 2>&1 | tail -2 || rc=1

echo
if [ "$rc" -eq 0 ]; then echo "VERIFICATION GATE: PASS"; else echo "VERIFICATION GATE: FAIL"; fi
exit "$rc"
