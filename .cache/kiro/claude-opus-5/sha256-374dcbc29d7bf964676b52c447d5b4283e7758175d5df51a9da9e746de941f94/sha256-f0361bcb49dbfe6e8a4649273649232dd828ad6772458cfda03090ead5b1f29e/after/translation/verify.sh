#!/usr/bin/env bash
# Full differential-verification run.
#
#   ./verify.sh
#
# Rebuilds the C shared library and the Rust cdylib, then runs the whole
# differential test suite against BOTH the release and the debug Rust `.so`, for
# every Cargo feature combination the crate declares.
#
# NOTE: `--test-threads=1` is mandatory. The `driver` tests compare stdout by
# temporarily redirecting file descriptor 1; libtest's own progress output goes
# to fd 1 as well, so a concurrently-running test thread would otherwise inject
# its "test <name> ... ok" line into the captured bytes.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

step "Building the C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
c_so="$root/c_src/build/libStaticLoop.so"

step "Building the Rust cdylib (release + debug)"
( cd "$here" && timeout 600 cargo build --release && timeout 600 cargo build ) \
  || { echo "Rust build FAILED"; exit 1; }

step "Symbol parity (nm -D --defined-only)"
c_syms=$(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort)
r_syms=$(nm -D --defined-only "$here/target/release/libStaticLoop.so" | awk '{print $NF}' | sort)
echo "C exports:"; echo "$c_syms" | sed 's/^/  /'
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST:"; echo "$missing" | sed 's/^/  /'; fail=1
else
  echo "OK: 0 C symbols missing from the Rust .so"
fi

step "Unresolved non-libc imports in the Rust .so"
sus=$(nm -D --undefined-only "$here/target/release/libStaticLoop.so" \
      | awk '{print $NF}' | grep -v '@' | grep -v '^_ITM_' | grep -v '^__')
if [ -n "$sus" ]; then echo "SUSPICIOUS: $sus"; fail=1; else echo "OK: none"; fi

# Feature combinations. The crate declares no [features]; the loop is derived
# from Cargo.toml so it stays correct if features are ever added.
step "Enumerating feature combinations from Cargo.toml"
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' "$here/Cargo.toml")
echo "declared features: ${feats:-<none>}"

run_suite() { # $1 = label, rest = cargo flags
  local label="$1"; shift
  step "Test suite: $label"
  ( cd "$here" && timeout 600 cargo test "$@" --no-fail-fast -- --test-threads=1 ) \
    || { echo "SUITE FAILED: $label"; fail=1; }
}

run_suite "default features / release .so" 
RUST_LIB_PATH="$here/target/debug/libStaticLoop.so" \
  run_suite "default features / debug .so"
run_suite "--no-default-features" --no-default-features
run_suite "--all-features" --all-features

for f in $feats; do
  run_suite "--no-default-features --features $f" --no-default-features --features "$f"
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
