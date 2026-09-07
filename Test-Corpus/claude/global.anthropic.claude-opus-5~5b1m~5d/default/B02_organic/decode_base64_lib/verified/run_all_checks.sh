#!/usr/bin/env bash
# Full verification sweep: builds the C .so, then runs every differential test
# against the Rust .so in every profile and every feature combination.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
OFFLINE="--offline"
rc=0

echo "=================================================================="
echo "0. Build the C shared library"
echo "=================================================================="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

echo
echo "=================================================================="
echo "1. Enumerate feature combinations from Cargo.toml"
echo "=================================================================="
# Extract declared features (excluding "default"). This crate declares none,
# so the loop degenerates to the single default configuration -- but it is
# derived mechanically rather than assumed.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> configurations: default, --no-default-features, --all-features"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "declared features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

echo
echo "=================================================================="
echo "2. Differential tests: every profile x every feature combination"
echo "=================================================================="
for profile in debug release; do
  if [ "$profile" = release ]; then PF="--release"; else PF=""; fi
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features='${combo:-<default>}'"
    echo "------------------------------------------------------------------"
    echo ">>> $label"
    # shellcheck disable=SC2086
    cargo build $PF $OFFLINE $combo >/dev/null 2>&1 || { echo "BUILD FAILED: $label"; rc=1; continue; }
    so="target/$profile/libdriver.so"
    if [ ! -f "$so" ]; then echo "MISSING $so"; rc=1; continue; fi
    # Point the harness at THIS profile's .so explicitly.
    # shellcheck disable=SC2086
    RUST_DRIVER_SO="$(pwd)/$so" timeout 600 cargo test $PF $OFFLINE $combo 2>&1 \
      | grep -E 'test result|FAILED|panicked|^error' \
      || { echo "TEST RUN PRODUCED NO RESULT: $label"; rc=1; }
    # shellcheck disable=SC2086
    RUST_DRIVER_SO="$(pwd)/$so" timeout 600 cargo test $PF $OFFLINE $combo >/dev/null 2>&1 \
      || { echo "!!! TESTS FAILED: $label"; rc=1; }
  done
done

echo
echo "=================================================================="
echo "3. Symbol parity (nm -D)"
echo "=================================================================="
c_syms=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '$2=="T"{print $3}' | sort)
r_syms=$(nm -D --defined-only target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
echo "C   exports: $(echo "$c_syms" | tr '\n' ' ')"
echo "Rust exports (T): $(echo "$r_syms" | tr '\n' ' ')"
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST .so:"; echo "$missing"; rc=1
else
  echo "SYMBOL DIFF EMPTY -- every C export is present in the Rust .so"
fi

echo
echo "=================================================================="
echo "4. Binary/driver executable check"
echo "=================================================================="
if grep -q add_executable "$ROOT/c_src/CMakeLists.txt"; then
  echo "C builds an executable -- stdout diff required!"; rc=1
else
  echo "c_src/CMakeLists.txt has no add_executable -> library only, no stdout to diff"
fi
if grep -q '^\[\[bin\]\]' Cargo.toml; then
  echo "Rust declares a [[bin]] -- stdout diff required!"; rc=1
else
  echo "Cargo.toml has no [[bin]] -> cdylib only"
fi

echo
echo "=================================================================="
if [ "$rc" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT (rc=$rc)"; fi
echo "=================================================================="
exit $rc
