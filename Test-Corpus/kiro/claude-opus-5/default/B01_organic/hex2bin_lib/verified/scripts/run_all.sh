#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the full differential suite
# across every feature combination declared in Cargo.toml.
#
# `cargo test` does NOT build a `crate-type = ["cdylib"]` artifact, so the
# cdylib must be built explicitly and its path handed to the tests via
# HEX2BIN_RUST_SO. Skipping this step is how a suite silently tests a stale .so.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"

TIMEOUT="${TIMEOUT:-600}"

echo "=== [1/4] building C shared library ==="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout "$TIMEOUT" cmake --build . >/dev/null )
c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
echo "C .so: $c_so"
test -f "$c_so"

echo
echo "=== [2/4] enumerating feature combinations from Cargo.toml ==="
mapfile -t combos < <(python3 - "$crate/Cargo.toml" <<'PY'
import itertools, sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
# Always test the default build and the no-default-features build.
print("__default__")
print("__nodefault__")
for r in range(1, len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(",".join(c))
PY
)
printf '  %s\n' "${combos[@]}"

echo
echo "=== [3/4] cargo check for every combination ==="
for combo in "${combos[@]}"; do
  case "$combo" in
    __default__)   args=() ;;
    __nodefault__) args=(--no-default-features) ;;
    *)             args=(--no-default-features --features "$combo") ;;
  esac
  echo "--- cargo check ${args[*]:-(default)}"
  ( cd "$crate" && timeout "$TIMEOUT" cargo check --all-targets "${args[@]}" )
done

echo
echo "=== [4/4] build cdylib + run tests for every combination ==="
fail=0
for combo in "${combos[@]}"; do
  case "$combo" in
    __default__)   args=() ;;
    __nodefault__) args=(--no-default-features) ;;
    *)             args=(--no-default-features --features "$combo") ;;
  esac
  echo
  echo "########## combination: ${combo} (${args[*]:-default}) ##########"
  ( cd "$crate" && timeout "$TIMEOUT" cargo build --release "${args[@]}" )
  rs_so="$crate/target/release/libhex2bin_lib.so"
  test -f "$rs_so"
  if ! ( cd "$crate" && HEX2BIN_RUST_SO="$rs_so" \
         timeout "$TIMEOUT" cargo test --release "${args[@]}" -- --test-threads=4 ); then
    echo "!!! FAILED for combination: ${combo}"
    fail=1
  fi
  # Also exercise the unoptimized build: different codegen, same required ABI.
  ( cd "$crate" && timeout "$TIMEOUT" cargo build "${args[@]}" )
  rs_so_dbg="$crate/target/debug/libhex2bin_lib.so"
  if ! ( cd "$crate" && HEX2BIN_RUST_SO="$rs_so_dbg" \
         timeout "$TIMEOUT" cargo test "${args[@]}" -- --test-threads=4 ); then
    echo "!!! FAILED (debug) for combination: ${combo}"
    fail=1
  fi
done

echo
echo "=== symbol parity ==="
"$here/symbol_diff.sh"

echo
echo "=== mutation check (proves the suite detects divergence) ==="
"$here/mutation_check.sh"

if [ "$fail" -ne 0 ]; then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all combinations passed"
