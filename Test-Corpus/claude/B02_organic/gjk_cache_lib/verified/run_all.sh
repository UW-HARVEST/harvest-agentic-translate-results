#!/usr/bin/env bash
# Full verification run: builds the C reference .so and the Rust cdylib, checks
# symbol parity, and runs the whole differential suite against BOTH the dev and
# the release Rust .so.
#
# Usage: ./run_all.sh
set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}"
fail=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Enumerate build configurations"
# Cargo.toml has no [features] table, so there is exactly one feature
# combination: the default (== --no-default-features). Verified mechanically:
FEATURES=$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | python3 -c 'import json,sys; print(",".join(json.load(sys.stdin)["packages"][0].get("features",{})))')
if [ -z "$FEATURES" ]; then
  echo "  no [features] in Cargo.toml -> 1 combination: <default>"
  COMBOS=("")
else
  echo "  features found: $FEATURES"
  COMBOS=("" "$FEATURES")
fi

# ---------------------------------------------------------------------------
step "Build C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$TMP/cmake.log" 2>&1 \
  && cmake --build . >>"$TMP/cmake.log" 2>&1 ) \
  && ok "libtranslated_rust.so" || { bad "C build (see $TMP/cmake.log)"; exit 1; }

# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  if timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} \
       >"$TMP/check.log" 2>&1; then
    ok "cargo check --features $label"
  else
    bad "cargo check --features $label"; tail -30 "$TMP/check.log"
  fi
done

# ---------------------------------------------------------------------------
step "Build Rust cdylib (dev + release)"
timeout 600 cargo build            >"$TMP/b1.log" 2>&1 && ok "dev"     || bad "dev build"
timeout 600 cargo build --release  >"$TMP/b2.log" 2>&1 && ok "release" || bad "release build"

# ---------------------------------------------------------------------------
step "Symbol parity (nm -D)"
C_SO=c_src/build/libtranslated_rust.so
for prof in debug release; do
  R_SO="target/$prof/libgjk_cache_lib.so"
  nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "$TMP/c_syms.txt"
  nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort > "$TMP/r_syms.txt"
  missing=$(comm -23 "$TMP/c_syms.txt" "$TMP/r_syms.txt")
  total=$(wc -l < "$TMP/c_syms.txt")
  if [ -z "$missing" ]; then
    ok "$prof: all $total C symbols exported by Rust"
  else
    bad "$prof: missing symbols:"; echo "$missing"
  fi
  # undefined symbols must be libc / unwinder only
  extra=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' || true)
  [ -z "$extra" ] && ok "$prof: no non-libc undefined symbols" \
                  || { bad "$prof: unexpected undefined symbols:"; echo "$extra"; }
done

# ---------------------------------------------------------------------------
step "Differential test suite"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  for prof in dev release; do
    if [ "$prof" = release ]; then
      export GJK_RUST_SO="$PWD/target/release/libgjk_cache_lib.so"
    else
      unset GJK_RUST_SO
    fi
    log="$TMP/test-$prof.log"
    if timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} \
         >"$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      ok "features=$label rust_so=$prof ($n tests)"
    else
      bad "features=$label rust_so=$prof"
      grep -E "^test .* FAILED|panicked|test result" "$log" | head -30
    fi
  done
done
unset GJK_RUST_SO

# ---------------------------------------------------------------------------
step "Summary"
if [ "$fail" = 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit $fail
