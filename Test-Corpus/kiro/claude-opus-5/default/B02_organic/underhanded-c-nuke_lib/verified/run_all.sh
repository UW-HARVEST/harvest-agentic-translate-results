#!/usr/bin/env bash
# Full verification run: rebuilds the C .so and the Rust cdylib, checks symbol
# parity, and runs every differential test under every feature combination and
# against both Rust build profiles.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

echo "=== 1. build the C shared object ==="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$(find "$root/c_src/build" -maxdepth 1 -name 'lib*.so' | head -n1)"
echo "C  .so: $c_so"

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# No [features] table => the only combinations are the default and
# --no-default-features (which are identical here). Derived, not hard-coded.
mapfile -t features < <(python3 - "$here/Cargo.toml" <<'PY'
import sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if line and not line.startswith('#') and '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
print('__default__')
print('__none__')
import itertools
for r in range(1, len(names) + 1):
    for c in itertools.combinations(names, r):
        print(','.join(c))
PY
)
printf '  %s\n' "${features[@]}"

for profile in release debug; do
  for combo in "${features[@]}"; do
    case "$combo" in
      __default__) flags=() ;;
      __none__)    flags=(--no-default-features) ;;
      *)           flags=(--no-default-features --features "$combo") ;;
    esac

    echo
    echo "=== 3. profile=$profile features=$combo ==="
    if [ "$profile" = release ]; then
      (cd "$here" && timeout 600 cargo build --release "${flags[@]}")
      rust_so="$here/target/release/libunderhanded_c_nuke_lib.so"
    else
      (cd "$here" && timeout 600 cargo build "${flags[@]}")
      rust_so="$here/target/debug/libunderhanded_c_nuke_lib.so"
    fi

    echo "--- symbol parity ---"
    diff <(nm -D --defined-only "$c_so"    | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort) \
      && echo "symbol diff: EMPTY (ok)"

    echo "--- undefined non-libc symbols in the Rust .so ---"
    nm -D --undefined-only "$rust_so" | awk '{print $NF}' \
      | grep -v -E '^(_ITM_|__cxa_|__gmon_|__tls_|_Unwind_)' \
      | grep -v -E '@GLIBC|@GCC' || true

    echo "--- differential tests ---"
    (cd "$here" && RUST_SO="$rust_so" C_SO="$c_so" \
      timeout 600 cargo test "${flags[@]}" -- --test-threads=1)
  done
done

echo
echo "=== ALL COMBINATIONS PASSED ==="
