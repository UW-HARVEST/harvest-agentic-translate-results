#!/usr/bin/env bash
# Full verification run: builds the C shared library, builds the Rust cdylib for
# every feature combination, and runs the Phase B / Phase C differential suites
# against both .so files.
#
# Usage: ./run_all.sh
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

echo "=== [1/4] building C shared library ==="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$root/c_src/build/libdriver.so"
test -f "$c_so"

# Enumerate feature combinations from Cargo.toml. The crate has no [features]
# section, so this yields the single default combination; the loop is written
# generically so new features are picked up automatically.
mapfile -t features < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' "$here/Cargo.toml" | grep -v '^default$' || true)

combos=("default")
if [ "${#features[@]}" -gt 0 ]; then
  combos+=("no-default")
  for f in "${features[@]}"; do combos+=("$f"); done
  combos+=("$(IFS=,; echo "${features[*]}")")
else
  combos+=("no-default")   # identical to default here, still exercised
fi

fail=0
for combo in "${combos[@]}"; do
  case "$combo" in
    default)    flags=() ;;
    no-default) flags=(--no-default-features) ;;
    *)          flags=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "=== [2/4] feature combo: ${combo} (${flags[*]:-<none>}) ==="
  (cd "$here" && timeout 600 cargo build --offline "${flags[@]}")
  rust_so="$here/target/debug/libdriver.so"
  test -f "$rust_so"

  echo "--- [3/4] symbol parity (${combo}) ---"
  c_syms=$(nm -D --defined-only "$c_so" | awk '{print $3}' | sort)
  r_syms=$(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms") || true)
  if [ -n "$missing" ]; then
    echo "FAIL: symbols exported by C but missing from Rust:"
    echo "$missing"
    fail=1
  else
    echo "OK: symbol diff empty ($(echo "$c_syms" | wc -l) symbols)"
  fi
  undef=$(nm -D --undefined-only "$rust_so" | awk '{print $2}' \
    | grep -v -E '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^gettid$|^statx$|^__cxa_' || true)
  if [ -n "$undef" ]; then
    echo "FAIL: unresolved non-libc symbols in Rust .so:"; echo "$undef"; fail=1
  else
    echo "OK: 0 missing/undefined non-libc symbols"
  fi

  echo "--- [4/4] differential tests (${combo}) ---"
  # fd 1/2 are hijacked by the capture harness, so tests must not run in parallel
  (cd "$here" && timeout 600 cargo test --offline "${flags[@]}" -- --test-threads=1) || fail=1

  echo "--- [4b/4] differential tests against the RELEASE .so (${combo}) ---"
  (cd "$here" && timeout 600 cargo build --release --offline "${flags[@]}")
  rel_so="$here/target/release/libdriver.so"
  test -f "$rel_so"
  rel_missing=$(comm -23 <(echo "$c_syms") \
    <(nm -D --defined-only "$rel_so" | awk '{print $3}' | sort) || true)
  if [ -n "$rel_missing" ]; then
    echo "FAIL: release .so missing symbols:"; echo "$rel_missing"; fail=1
  else
    echo "OK: release symbol diff empty"
  fi
  (cd "$here" && RUST_DRIVER_SO="$rel_so" \
     timeout 600 cargo test --offline "${flags[@]}" -- --test-threads=1) || fail=1
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; exit 1; fi
