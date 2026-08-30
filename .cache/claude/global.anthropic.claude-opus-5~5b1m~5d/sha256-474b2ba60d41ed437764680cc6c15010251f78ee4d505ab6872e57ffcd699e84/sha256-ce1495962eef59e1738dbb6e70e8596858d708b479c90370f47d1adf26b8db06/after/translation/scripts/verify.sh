#!/usr/bin/env bash
# One-shot full verification: everything the completion gate requires.
#
#   1. build the C shared library with cmake
#   2. cargo check / build the Rust cdylib
#   3. symbol parity: nm -D diff must be empty
#   4. Phases B + C + D differential suites, under every feature combination
#      and both cargo profiles
#   5. harness self-validation (mutation testing)
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="--offline"
rc=0
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
fail() { echo "FAILED: $*"; rc=1; }

step "1/5  build the C shared library"
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) || fail "cmake"
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO" || fail "no C .so"

step "2/5  cargo check + build the Rust cdylib (dev and release)"
cargo check $CARGO_FLAGS --all-targets 2>&1 | grep -E "^error" && fail "cargo check"
cargo build $CARGO_FLAGS >/dev/null || fail "cargo build (dev)"
cargo build $CARGO_FLAGS --release >/dev/null || fail "cargo build (release)"

step "3/5  symbol parity (nm -D)"
for prof in debug release; do
  R_SO="target/$prof/libdriver.so"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWDBRVi]$/ {print $3}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TWDBRVi]$/ {print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  $prof: MISSING from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail "symbol parity ($prof)"
  else
    echo "  $prof: 0 missing symbols  ($(nm -D --defined-only "$C_SO" | grep -c ' T ') C exports all present)"
  fi
  # No unresolvable non-libc imports (dlopen with RTLD_NOW would fail).
  unres=$(nm -D --undefined-only "$R_SO" | awk '$1=="U"{print $2}' |
    grep -vE '@GLIBC_|@GCC_|^_Unwind_|^__' || true)
  [ -n "$unres" ] && { echo "  $prof: unresolved: $unres"; fail "unresolved imports ($prof)"; }
done

step "4/5  Phases B + C + D across every feature combo and both profiles"
bash scripts/check_features.sh || fail "differential suites"

step "5/5  harness self-validation (mutation testing)"
bash scripts/mutation_check.sh || fail "mutation testing"

step "RESULT"
if [ $rc -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "ONE OR MORE CHECKS FAILED"
fi
exit $rc
