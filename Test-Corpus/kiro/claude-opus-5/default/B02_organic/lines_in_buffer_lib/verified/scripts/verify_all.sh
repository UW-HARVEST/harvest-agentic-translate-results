#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for EVERY build-time
# configuration.
#
# Configurations are derived from:
#   * translation/Cargo.toml [features]  -> Rust feature combinations
#   * c_src/CMakeLists.txt               -> C compile-time options
# Neither declares any, so the single valid configuration is "no features".
# The loop below is still generated from Cargo.toml so it keeps working if
# features are added later.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

# ---------------------------------------------------------------- C reference
echo "== building C reference =="
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
  && cmake --build . >>/tmp/cmake.log 2>&1)
c_so="$root/c_src/build/libdriver.so"

# ------------------------------------------------- enumerate feature combos
mapfile -t features < <(
  python3 - <<'PY'
import re, sys
txt = open('translation/Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            n = line.split('=', 1)[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print('\n'.join(names))
PY
)

combos=("")
for f in "${features[@]}"; do
  [[ -z "$f" ]] && continue
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    new+=("${c:+$c,}$f")
  done
  combos=("${new[@]}")
done

echo "== ${#combos[@]} feature combination(s): ${combos[*]:-<none>} =="

# ------------------------------------------------------------- check + test
for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "=================================================================="
  echo "== configuration: $label"
  echo "=================================================================="

  export FFI_NO_DEFAULT_FEATURES=1
  export FFI_FEATURES="$combo"

  ( cd translation && timeout 600 cargo check --all-targets \
      --no-default-features ${combo:+--features "$combo"} )

  ( cd translation && timeout 600 cargo build --release --lib \
      --target-dir target/ffi-cdylib \
      --no-default-features ${combo:+--features "$combo"} )
  rust_so="$root/translation/target/ffi-cdylib/release/libdriver.so"

  echo "-- symbol parity (every symbol the C .so exports must exist in Rust) --"
  c_syms=$(nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms") || true)
  if [[ -n "$missing" ]]; then
    echo "MISSING EXPORTS in Rust .so for $label:"
    echo "$missing"
    exit 1
  fi
  echo "ok: $(echo "$c_syms" | wc -l) C symbol(s) all present in the Rust .so"

  ( cd translation && timeout 600 cargo test \
      --no-default-features ${combo:+--features "$combo"} )
done

echo
echo "ALL CONFIGURATIONS PASSED"
