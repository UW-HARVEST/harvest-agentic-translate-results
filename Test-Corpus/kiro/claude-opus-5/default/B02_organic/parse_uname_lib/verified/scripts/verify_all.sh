#!/usr/bin/env bash
# The whole completion gate, in order. Run from translation/.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
ROOT=$(cd .. && pwd)

fail=0
step() { echo; echo "########## $* ##########"; }

step "0. build the C reference shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) | tail -2 || fail=1

step "1. cargo check"
timeout 600 cargo check 2>&1 | tail -3 || fail=1

step "2. build the Rust shared library (debug + release)"
timeout 600 cargo build --quiet && timeout 600 cargo build --release --quiet || fail=1
ls -l target/debug/libdriver.so target/release/libdriver.so

step "3. SYMBOLS.md — symbol parity (nm -D)"
bash scripts/symbol_diff.sh || fail=1

step "4. Phases B, C, D — the differential suite"
timeout 600 cargo test 2>&1 | grep -E "^(running|test result|error)" || fail=1

step "5. ERRORS.md / CONFIGS.md — every row maps to a passing test"
timeout 600 python3 scripts/audit_artifacts.py | tail -5 || fail=1

step "6. every feature combination"
timeout 900 bash scripts/feature_matrix.sh | tail -3 || fail=1

step "7. the suite can actually detect divergence (mutation check)"
timeout 900 python3 scripts/mutation_check.py | tail -3 || fail=1

step "8. c_src/ untouched"
if git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  changed=$(git -C "$ROOT" status --porcelain -- c_src | grep -v 'c_src/build' || true)
  if [ -n "$changed" ]; then echo "MODIFIED: $changed"; fail=1; else echo "clean"; fi
else
  echo "not a git repo; c_src source mtimes:"
  find "$ROOT/c_src" -name '*.c' -o -name '*.h' -o -name 'CMakeLists.txt' | xargs ls -l
fi

echo
if [ "$fail" -ne 0 ]; then echo "GATE: FAIL"; exit 1; fi
echo "GATE: PASS"
