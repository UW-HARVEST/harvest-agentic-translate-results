#!/usr/bin/env bash
# Differential verification driver.
#
#  1. builds the C reference shared library (never modifies c_src sources)
#  2. discovers every feature combination declared in Cargo.toml
#  3. for each combination: builds the Rust cdylib for the profile under test,
#     then runs the phase B/C/D differential test suites
#  4. prints the nm -D symbol diff
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
c_build="$root/c_src/build"
fail=0

echo "=== 1. build C reference .so ==="
( cd "$root/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$c_build/libdriver.so"

echo
echo "=== 2. discover feature combinations ==="
# Every feature name in the [features] table (none => default build only).
features="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' "$here/Cargo.toml")"
combos=()
if [[ -z "${features// }" ]]; then
  echo "no [features] table -> single configuration: default"
  combos=("__default__")
else
  # default, each feature alone, no-default-features, and the full set
  combos=("__default__" "__nodefault__")
  for f in $features; do combos+=("$f"); done
  combos+=("$(echo "$features" | paste -sd, -)")
fi
printf 'combination: %s\n' "${combos[@]}"

echo
echo "=== 3. run differential suites per combination ==="
for combo in "${combos[@]}"; do
  case "$combo" in
    __default__)   flags=() ; label="default" ;;
    __nodefault__) flags=(--no-default-features) ; label="no-default-features" ;;
    *)             flags=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac
  echo "--- [$label] cargo build (cdylib must exist for the test profile) ---"
  ( cd "$here" && timeout 600 cargo build "${flags[@]}" ) || { echo "[$label] BUILD FAILED"; fail=1; continue; }
  echo "--- [$label] cargo test ---"
  ( cd "$here" && timeout 600 cargo test "${flags[@]}" ) || { echo "[$label] TESTS FAILED"; fail=1; }
done

echo
echo "=== 4. nm -D symbol diff (C vs Rust) ==="
( cd "$here" && timeout 600 cargo build --release >/dev/null 2>&1 )
c_syms="$(nm -D --defined-only "$c_build/libdriver.so" | awk '{print $NF}' | sed 's/@.*//' | sort -u)"
for prof in debug release; do
  so="$here/target/$prof/libdriver.so"
  [[ -f "$so" ]] || { echo "($prof cdylib not built, skipped)"; continue; }
  r_syms="$(nm -D --defined-only "$so" | awk '{print $NF}' | sed 's/@.*//' | sort -u)"
  missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
  if [[ -n "$missing" ]]; then
    echo "MISSING from Rust $prof .so:"; echo "$missing"; fail=1
  else
    echo "$prof: OK - all $(echo "$c_syms" | wc -l) C symbol(s) exported by Rust .so"
  fi
  echo "  unresolved deps:"; ldd "$so" | grep -c "not found" | sed 's/^/    not-found count: /'
done

echo
if [[ $fail -eq 0 ]]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit $fail
