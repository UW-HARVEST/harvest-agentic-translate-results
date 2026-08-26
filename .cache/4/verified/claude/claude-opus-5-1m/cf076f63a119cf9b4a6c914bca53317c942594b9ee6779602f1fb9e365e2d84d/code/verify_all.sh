#!/bin/bash
# Phase D driver: every feature combination, end to end.
#
#   1. cargo check   for each combination
#   2. cargo test    for each combination (Phases B + C)
#   3. nm -D symbol-parity diff, C .so vs Rust .so, for each combination
#
# `Cargo.toml` has one non-default feature (`diff_internals`, test-only), so the
# power set is: {} and {diff_internals}.
set -u
cd "$(dirname "$0")" || exit 1
ROOT=$PWD
FAIL=0

# --- enumerate the feature power set straight out of Cargo.toml --------------
FEATS=$(python3 - <<'PY'
import itertools, re
body = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', body, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
for r in range(len(names) + 1):
    for combo in itertools.combinations(names, r):
        print(','.join(combo))
PY
)

echo "Feature combinations to verify:"
while IFS= read -r f; do echo "  - '${f:-<none>}'"; done <<< "$FEATS"
echo

# --- build the C side once ---------------------------------------------------
echo "=== Building C library (CMake) ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$ROOT/c_src/build/libtranslated_rust.so
echo "  $C_SO"

echo "=== Building C shim (reaches the 14 static routines) ==="
mkdir -p target/diffshim
gcc -shared -fPIC -I c_src/include -o target/diffshim/libcshim.so tests/cshim/cshim.c \
  || { echo "C shim build FAILED"; exit 1; }
C_SHIM=$ROOT/target/diffshim/libcshim.so
echo "  $C_SHIM"
echo

# --- per-combination ---------------------------------------------------------
while IFS= read -r f; do
  label=${f:-<none>}
  if [ -z "$f" ]; then FARGS=(); else FARGS=(--features "$f"); fi
  echo "############################################################"
  echo "### FEATURES: $label"
  echo "############################################################"

  echo "--- cargo check ---"
  if ! timeout 600 cargo check --offline --no-default-features "${FARGS[@]}" --all-targets \
        > "$TMPDIR/check.$$.log" 2>&1; then
    echo "  CHECK FAILED"; tail -30 "$TMPDIR/check.$$.log"; FAIL=1; continue
  fi
  warn=$(grep -c '^warning' "$TMPDIR/check.$$.log")
  echo "  ok (warnings: $warn)"

  echo "--- cargo test (Phase B + Phase C) ---"
  if timeout 600 cargo test --offline --no-default-features "${FARGS[@]}" \
       > "$TMPDIR/test.$$.log" 2>&1; then
    grep -E '^test result:' "$TMPDIR/test.$$.log" | sed 's/^/  /'
  else
    echo "  TESTS FAILED"; grep -E '^(test result|failures:|---- |thread)' -A3 "$TMPDIR/test.$$.log" | head -40
    FAIL=1
  fi

  echo "--- Phase D: nm -D symbol parity ---"
  # Under diff_internals the Rust .so gains the diffshim_* wrappers, so it is
  # compared against the C *shim* (which exports the same set). With no
  # features it is compared against the real C library.
  RS_SO=$ROOT/target/difftest/debug/libget_predict_func_lib.so
  if [ -z "$f" ]; then REF=$C_SO; else REF=$C_SHIM; fi
  nm -D --defined-only "$REF"   | awk '{print $3}' | grep -v '^$' | sort > "$TMPDIR/c.$$.sym"
  nm -D --defined-only "$RS_SO" | awk '{print $3}' | grep -v '^$' | sort > "$TMPDIR/r.$$.sym"
  echo "  C   ($(basename "$REF")): $(wc -l < "$TMPDIR/c.$$.sym") exported"
  echo "  Rust: $(wc -l < "$TMPDIR/r.$$.sym") exported"
  missing=$(comm -23 "$TMPDIR/c.$$.sym" "$TMPDIR/r.$$.sym")
  extra=$(comm -13 "$TMPDIR/c.$$.sym" "$TMPDIR/r.$$.sym")
  if [ -n "$missing" ]; then
    echo "  *** MISSING from Rust .so:"; echo "$missing" | sed 's/^/      /'; FAIL=1
  else
    echo "  missing from Rust: none"
  fi
  if [ -n "$extra" ]; then
    echo "  note: extra in Rust .so:"; echo "$extra" | sed 's/^/      /'
  else
    echo "  extra in Rust: none"
  fi

  echo "  undefined non-libc symbols in Rust .so:"
  nm -D --undefined-only "$RS_SO" | awk '{print $2}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' \
    | grep -v '^$' | sed 's/^/      /' || true
  echo "      (empty above = all imports are libc/unwind runtime)"
  echo
done <<< "$FEATS"

echo "############################################################"
if [ "$FAIL" -eq 0 ]; then
  echo "### ALL FEATURE COMBINATIONS PASSED"
else
  echo "### FAILURES PRESENT"
fi
echo "############################################################"
exit $FAIL
