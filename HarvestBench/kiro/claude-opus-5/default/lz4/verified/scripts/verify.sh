#!/bin/bash
# Full verification run: build both libraries, diff their exported symbols,
# enumerate every Cargo feature combination, and run the differential suite
# under each one.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== 1. build the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . > /dev/null ) || { echo "C build FAILED"; exit 1; }

echo "=== 2. build the Rust shared library ==="
timeout 600 cargo build --release > /tmp/lz4-rustbuild.log 2>&1 \
  || { echo "Rust build FAILED"; tail -30 /tmp/lz4-rustbuild.log; exit 1; }

C_SO="$ROOT/c_src/build/liblz4.so"
R_SO="target/release/liblz4.so"

echo "=== 3. symbol parity ==="
nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/lz4-c.syms
nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/lz4-r.syms
echo "C exports:    $(wc -l < /tmp/lz4-c.syms)"
echo "Rust exports: $(wc -l < /tmp/lz4-r.syms)"
MISSING=$(comm -23 /tmp/lz4-c.syms /tmp/lz4-r.syms)
EXTRA=$(comm -13 /tmp/lz4-c.syms /tmp/lz4-r.syms)
if [ -n "$MISSING" ]; then echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1; else echo "missing: none"; fi
if [ -n "$EXTRA" ];   then echo "EXTRA IN RUST:";    echo "$EXTRA";   else echo "extra:   none"; fi

echo "--- undefined non-libc symbols in the Rust .so ---"
UNDEF=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
  | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|fread|free|fstat64|fwrite|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)')
if [ -n "$UNDEF" ]; then echo "UNRESOLVED:"; echo "$UNDEF"; FAIL=1; else echo "none"; fi

echo "=== 4. feature combinations ==="
FEATS=$(./scripts/features.sh)
if [ "$FEATS" = "NO_FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only configuration is the default set."
  COMBOS=("default")
else
  # power set of the declared features
  mapfile -t F <<< "$FEATS"
  n=${#F[@]}
  COMBOS=()
  for ((m=0; m<(1<<n); m++)); do
    c=""
    for ((b=0; b<n; b++)); do
      if (( m & (1<<b) )); then c="$c,${F[b]}"; fi
    done
    COMBOS+=("${c#,}")
  done
fi

for combo in "${COMBOS[@]}"; do
  echo
  if [ "$combo" = "default" ]; then
    echo "--- features: <default> ---"
    ARGS=(--release)
  elif [ -z "$combo" ]; then
    echo "--- features: <none> ---"
    ARGS=(--release --no-default-features)
  else
    echo "--- features: $combo ---"
    ARGS=(--release --no-default-features --features "$combo")
  fi
  timeout 600 cargo build "${ARGS[@]}" > /tmp/lz4-b.log 2>&1 \
    || { echo "  build FAILED"; tail -20 /tmp/lz4-b.log; FAIL=1; continue; }
  for t in lz4_block lz4hc xxhash lz4frame_valid lz4frame_errors lz4file cross_impl; do
    printf '  %-18s ' "$t"
    if timeout 600 cargo test "${ARGS[@]}" --test "$t" -- --test-threads=1 \
         > "/tmp/lz4-$t.log" 2>&1; then
      grep -h "^test result" "/tmp/lz4-$t.log" | tail -1
    else
      echo "FAILED"
      grep -E "DIVERGENCE|panicked|^test .* FAILED|SIGSEGV" "/tmp/lz4-$t.log" | head -5
      FAIL=1
    fi
  done
done

echo
if [ "$FAIL" = 0 ]; then echo "=== VERIFICATION PASSED ==="; else echo "=== VERIFICATION FAILED ==="; fi
exit $FAIL
