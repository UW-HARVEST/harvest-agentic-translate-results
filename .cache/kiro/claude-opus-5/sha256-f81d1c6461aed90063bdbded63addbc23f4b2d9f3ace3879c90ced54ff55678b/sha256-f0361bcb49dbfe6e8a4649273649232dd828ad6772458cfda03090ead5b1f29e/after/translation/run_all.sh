#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the exported symbol
# sets, and runs the differential test suite under every feature combination
# declared in Cargo.toml.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
C_SO="$ROOT/c_src/build/libSieve.so"
R_SO="$HERE/target/release/libSieve.so"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$C_SO"

# --------------------------------------------------------------------------
# NOTE: `--lib` is mandatory here. `cargo build --examples` alone does NOT
# rebuild the cdylib, which would leave the tests verifying a stale artifact.
step "Build the Rust cdylib + test runner"
( cd "$HERE" && timeout 600 cargo build --release --lib --examples ) \
  || { echo "Rust build FAILED"; exit 1; }
ls -l "$R_SO" "$HERE/target/release/examples/runner"

# --------------------------------------------------------------------------
step "Symbol parity (nm -D --defined-only)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/sieve-c-syms.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/sieve-r-syms.txt
echo "C exports:    $(tr '\n' ' ' < /tmp/sieve-c-syms.txt)"
echo "Rust exports: $(tr '\n' ' ' < /tmp/sieve-r-syms.txt)"
missing="$(comm -23 /tmp/sieve-c-syms.txt /tmp/sieve-r-syms.txt)"
extra="$(comm -13 /tmp/sieve-c-syms.txt /tmp/sieve-r-syms.txt)"
if [[ -n "$missing" ]]; then echo "MISSING from Rust .so: $missing"; fail=1; else echo "missing: none"; fi
if [[ -n "$extra" ]];   then echo "EXTRA in Rust .so: $extra";      fail=1; else echo "extra:   none"; fi

step "Undefined non-libc symbols in the Rust .so"
# Everything the Rust .so imports must resolve from libc / libgcc, both already
# present in any process that loads it.
undef="$(nm -D -u "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u)"
unresolved="$(ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' || true)"
echo "$undef" | tr '\n' ' '; echo
if [[ -n "$unresolved" ]]; then echo "UNRESOLVED: $unresolved"; fail=1; else echo "ldd -r: all imports resolve"; fi

# --------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
features="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "") print a[1]}
' "$HERE/Cargo.toml")"
if [[ -z "$features" ]]; then
  echo "no [features] section: the only build configuration is the default (empty) feature set"
  combos=("" "--no-default-features")
else
  combos=("" "--no-default-features" "--all-features")
  while read -r f; do
    [[ -n "$f" ]] && combos+=("--no-default-features --features $f")
  done <<< "$features"
fi

for combo in "${combos[@]}"; do
  step "cargo check ${combo:-(default)}"
  ( cd "$HERE" && timeout 600 cargo check --release $combo ) || { echo "check FAILED"; fail=1; }
done

for combo in "${combos[@]}"; do
  step "cargo test ${combo:-(default)}"
  ( cd "$HERE" && timeout 600 cargo build --release --lib --examples $combo >/dev/null ) \
    || { echo "build FAILED"; fail=1; continue; }
  ( cd "$HERE" && timeout 600 cargo test --release $combo ) || { echo "test FAILED"; fail=1; }
done

# --------------------------------------------------------------------------
step "RESULT"
if [[ $fail -eq 0 ]]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $fail
