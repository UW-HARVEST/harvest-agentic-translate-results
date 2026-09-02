#!/usr/bin/env bash
# Phase D — symbol parity + every feature combination.
#
#   ./scripts/verify.sh
#
# 1. builds the C shared library
# 2. builds the Rust cdylib
# 3. diffs `nm -D` between the two (must be empty both ways)
# 4. enumerates the crate's feature combinations and runs `cargo check` and the
#    full differential suite for each
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(cd .. && pwd)
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT
rc=0

echo "== 1. building the C shared library =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FAIL: C build"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "   $C_SO"

echo "== 2. building the Rust cdylib =="
timeout 600 cargo build --release >/dev/null 2>&1 || { echo "FAIL: cargo build"; exit 1; }
R_SO=target/release/libintput_lib.so
echo "   $R_SO"

echo "== 3. nm -D symbol parity =="
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > "$OUT/c.txt"
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > "$OUT/r.txt"
missing=$(comm -23 "$OUT/c.txt" "$OUT/r.txt")
extra=$(comm -13 "$OUT/c.txt" "$OUT/r.txt")
echo "   C exports:    $(wc -l < "$OUT/c.txt")"
echo "   Rust exports: $(wc -l < "$OUT/r.txt")"
if [ -n "$missing" ]; then echo "FAIL: missing from Rust:"; echo "$missing"; rc=1; fi
if [ -n "$extra" ];   then echo "FAIL: extra in Rust:";   echo "$extra";   rc=1; fi
[ -z "$missing$extra" ] && echo "   symbol diff is EMPTY"

# undefined symbols must all be libc / libgcc-unwind / loader
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sort -u > "$OUT/undef.txt"
bad=$(grep -v -E '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|getcwd|getenv|gettid|lseek|malloc|memcpy|memmove|memset|mmap|munmap|open|posix_memalign|pthread_|read|readlink|realloc|realpath|stat|statx|strcmp|strlen|syscall|write)' "$OUT/undef.txt" || true)
if [ -n "$bad" ]; then echo "FAIL: non-libc undefined symbols:"; echo "$bad"; rc=1; else
  echo "   0 non-libc undefined symbols"; fi

echo "== 4. feature combinations =="
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print('\n'.join(names))
PY
)

combos=("--no-default-features" "")
if [ "${#FEATURES[@]}" -gt 0 ] && [ -n "${FEATURES[0]:-}" ]; then
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[$b]}")
    done
    if [ "${#sel[@]}" -gt 0 ]; then
      combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  echo "   declared features: ${FEATURES[*]}"
else
  echo "   no [features] section in Cargo.toml -> only the default configuration"
fi

for c in "${combos[@]}"; do
  label=${c:-"(default)"}
  if ! timeout 600 cargo check $c >/dev/null 2>&1; then
    echo "FAIL: cargo check $label"; rc=1; continue
  fi
  if ! timeout 600 cargo build --release $c >/dev/null 2>&1; then
    echo "FAIL: cargo build --release $label"; rc=1; continue
  fi
  # re-check symbol parity for this configuration
  nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > "$OUT/r2.txt"
  if [ -n "$(comm -3 "$OUT/c.txt" "$OUT/r2.txt")" ]; then
    echo "FAIL: symbol parity broken under $label"; rc=1; continue
  fi
  if timeout 600 cargo test --release --tests $c >/dev/null 2>&1; then
    echo "   PASS  cargo test --release $label"
  else
    echo "FAIL: cargo test --release $label"; rc=1
  fi
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "PHASE D FAILURES (rc=$rc)"; fi
exit $rc
