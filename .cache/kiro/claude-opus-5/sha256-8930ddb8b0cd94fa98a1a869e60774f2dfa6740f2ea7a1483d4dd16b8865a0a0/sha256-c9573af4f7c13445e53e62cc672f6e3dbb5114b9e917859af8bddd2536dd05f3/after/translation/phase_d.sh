#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml
# and run the full differential suite under each, in both profiles, then diff
# `nm -D` between the C and Rust shared objects.
set -uo pipefail
cd "$(dirname "$0")"

FAIL=0

# ---- enumerate features mechanically from Cargo.toml ----------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

echo "=== declared features: [${FEATURES:-<none>}] ==="

# Build the combination list: always the default and --no-default-features;
# plus the powerset of declared features if there are any.
COMBOS=()
COMBOS+=("--no-default-features")
COMBOS+=("")            # default
if [ -n "$FEATURES" ]; then
  COMBOS+=("--all-features")
  set -- $FEATURES
  n=$#
  total=$((1 << n))
  for ((mask=1; mask<total; mask++)); do
    list=""
    for ((i=0; i<n; i++)); do
      if (( (mask >> i) & 1 )); then
        eval "f=\${$((i+1))}"
        list="${list:+$list,}$f"
      fi
    done
    COMBOS+=("--no-default-features --features $list")
  done
fi

# ---- cargo check + test every combination in both profiles ----------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  for prof in "" "--release"; do
    plabel="${prof:-debug}"
    echo "--- cargo check  $label  [$plabel] ---"
    if ! timeout 600 cargo check $combo $prof >/tmp/pd_check.log 2>&1; then
      echo "CHECK FAILED: $label [$plabel]"; tail -20 /tmp/pd_check.log; FAIL=1; continue
    fi
    echo "--- cargo test   $label  [$plabel] ---"
    if ! timeout 600 cargo test $combo $prof >/tmp/pd_test.log 2>&1; then
      echo "TEST FAILED: $label [$plabel]"; tail -40 /tmp/pd_test.log; FAIL=1
    else
      grep -E "^test result:" /tmp/pd_test.log | sed 's/^/    /'
    fi
  done
done

# ---- symbol parity --------------------------------------------------------
echo "=== symbol parity (nm -D) ==="
ROOT="$(cd .. && pwd)"
C_SO=$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)
R_SO=$(ls target/so-under-test/release/libcrc16_lib.so target/release/libcrc16_lib.so 2>/dev/null | head -1)
echo "C   : $C_SO"
echo "Rust: $R_SO"

if [ -z "$C_SO" ] || [ -z "$R_SO" ]; then
  echo "MISSING .so — cannot diff symbols"; FAIL=1
else
  nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/pd_c_syms.txt
  nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > /tmp/pd_r_syms.txt
  echo "C exports  : $(wc -l < /tmp/pd_c_syms.txt)"
  echo "Rust exports: $(wc -l < /tmp/pd_r_syms.txt)"
  MISSING=$(comm -23 /tmp/pd_c_syms.txt /tmp/pd_r_syms.txt)
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
  else
    echo "OK: 0 C symbols missing from the Rust .so"
  fi
  # undefined non-libc symbols in the Rust .so
  UNDEF=$(nm -D -u "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_Unwind_|statx|gettid)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)$' \
    | sort -u)
  if [ -n "$UNDEF" ]; then
    echo "UNDEFINED non-libc symbols in the Rust .so:"; echo "$UNDEF"; FAIL=1
  else
    echo "OK: 0 undefined non-libc symbols in the Rust .so"
  fi
fi

# ---- table-data parity ----------------------------------------------------
echo "=== CRC table parity (C header vs tables.rs) ==="
awk '/tflac_crc16_tables\[8\]\[256\]/,/^};/' "$ROOT/c_src/include/lib.h" \
  | grep -oE '0x[0-9a-fA-F]{4}' | tr 'A-F' 'a-f' > /tmp/pd_c_tab.txt
grep -oE '0x[0-9a-fA-F]{4}' src/tables.rs | tr 'A-F' 'a-f' > /tmp/pd_r_tab.txt
echo "C values: $(wc -l < /tmp/pd_c_tab.txt)  Rust values: $(wc -l < /tmp/pd_r_tab.txt)"
if diff -q /tmp/pd_c_tab.txt /tmp/pd_r_tab.txt >/dev/null; then
  echo "OK: 2048 table values identical"
else
  echo "TABLE MISMATCH"; diff /tmp/pd_c_tab.txt /tmp/pd_r_tab.txt | head -20; FAIL=1
fi

echo
if [ "$FAIL" -eq 0 ]; then echo "PHASE D: ALL CHECKS PASSED"; else echo "PHASE D: FAILURES PRESENT"; fi
exit $FAIL
