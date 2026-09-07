#!/usr/bin/env bash
# Full verification run: builds the C .so, builds the Rust cdylib, diffs the
# exported symbol tables, then runs every phase's differential tests under
# every feature combination declared in Cargo.toml.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

echo "== building the C shared library =="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
c_so="$(ls "$root"/c_src/build/*.so | head -1)"
echo "   $c_so"

echo "== building the Rust cdylib =="
cd "$here"
cargo build --offline --release >/dev/null
r_so="$here/target/release/libintput_lib.so"
echo "   $r_so"

echo "== symbol parity (nm -D --defined-only) =="
nm -D --defined-only "$c_so" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
nm -D --defined-only "$r_so" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/r_syms.txt"
if diff -u "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt"; then
  echo "   OK: $(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") symbols, 0 missing, 0 extra"
else
  echo "   FAIL: symbol tables differ" >&2
  exit 1
fi

echo "== feature combinations =="
combos="$(python3 - <<'EOF'
import tomllib
f = tomllib.load(open('Cargo.toml','rb')).get('features', {})
names = [k for k in f if k != 'default']
print(' '.join(names) if names else '')
EOF
)"
if [ -z "$combos" ]; then
  echo "   no [features] declared -> the default configuration is the only one"
  sets=("default")
else
  echo "   features: $combos"
  sets=("default" $combos)
fi

for s in "${sets[@]}"; do
  echo "== tests (feature set: $s) =="
  if [ "$s" = "default" ]; then
    timeout 600 cargo test --offline -- --test-threads=1
    timeout 600 cargo test --offline --no-default-features -- --test-threads=1
  else
    timeout 600 cargo test --offline --no-default-features --features "$s" -- --test-threads=1
  fi
done

echo
echo "ALL PHASES PASSED"
