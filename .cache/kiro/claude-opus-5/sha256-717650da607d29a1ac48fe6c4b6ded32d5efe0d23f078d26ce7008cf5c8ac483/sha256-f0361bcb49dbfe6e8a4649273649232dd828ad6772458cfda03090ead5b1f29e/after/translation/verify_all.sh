#!/usr/bin/env bash
# Phase D driver: run every phase under every feature combination and profile,
# then diff the exported symbols of the C and Rust shared objects.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE" || exit 1

# ---- enumerate feature combinations from Cargo.toml (powerset) -------------
mapfile -t FEATS < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "declared features: ${#FEATS[@]} ${FEATS[*]:-(none)}"

COMBOS=()
COMBOS+=("--all-features")
COMBOS+=("")                      # default features
COMBOS+=("--no-default-features")
n=${#FEATS[@]}
if (( n > 0 && n <= 12 )); then
  for ((mask=1; mask < (1<<n); mask++)); do
    sel=""
    for ((b=0; b<n; b++)); do
      (( mask & (1<<b) )) && sel="${sel:+$sel,}${FEATS[b]}"
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi

FAIL=0

for PROFILE in "" "--release"; do
  for COMBO in "${COMBOS[@]}"; do
    label="profile='${PROFILE:-debug}' combo='${COMBO:-<default>}'"
    echo "=============================================================="
    echo ">>> cargo check   $label"
    if ! timeout 600 cargo check $PROFILE $COMBO >/tmp/pd_check.log 2>&1; then
      echo "!!! CHECK FAILED  $label"; tail -20 /tmp/pd_check.log; FAIL=1; continue
    fi

    # Build the cdylib for THIS profile so the harness tests the matching .so
    # (it refuses to fall back to the other profile).
    if ! timeout 600 cargo build $PROFILE $COMBO >/tmp/pd_build.log 2>&1; then
      echo "!!! BUILD FAILED  $label"; tail -20 /tmp/pd_build.log; FAIL=1; continue
    fi

    echo ">>> cargo test    $label"
    if ! timeout 600 cargo test $PROFILE $COMBO >/tmp/pd_test.log 2>&1; then
      echo "!!! TEST FAILED   $label"
      grep -E 'DIVERGENCE|EXHAUSTIVE|^test .* FAILED|test result' /tmp/pd_test.log | head -30
      FAIL=1; continue
    fi
    grep -E 'test result' /tmp/pd_test.log | sed 's/^/    /'
  done
done

# ---- cross-profile matrix: every test binary x every cdylib build ---------
echo "=============================================================="
echo ">>> cross-profile matrix (test binary profile x cdylib profile)"
for TP in "" "--release"; do
  for SOP in debug release; do
    SO="$CRATE/target/$SOP/libfloat2half_lib.so"
    [ -f "$SO" ] || { echo "!!! missing $SO"; FAIL=1; continue; }
    echo "--- test-binary='${TP:-debug}' cdylib='$SOP'"
    if ! FLOAT2HALF_RUST_SO="$SO" timeout 600 cargo test $TP >/tmp/pd_x.log 2>&1; then
      echo "!!! FAILED test='${TP:-debug}' cdylib='$SOP'"
      grep -E 'DIVERGENCE|EXHAUSTIVE|^test .* FAILED|test result' /tmp/pd_x.log | head -20
      FAIL=1
    else
      grep -E 'test result' /tmp/pd_x.log | sed 's/^/    /'
    fi
  done
done

# ---- symbol parity --------------------------------------------------------
echo "=============================================================="
echo ">>> Phase D symbol parity"
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
R_SO="$CRATE/target/release/libfloat2half_lib.so"
[ -f "$R_SO" ] || { echo "!!! missing $R_SO"; FAIL=1; }

nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/pd_c_syms.txt
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > /tmp/pd_r_syms.txt
echo "C exports:    $(wc -l < /tmp/pd_c_syms.txt)"
echo "Rust exports: $(wc -l < /tmp/pd_r_syms.txt)"

MISSING=$(comm -23 /tmp/pd_c_syms.txt /tmp/pd_r_syms.txt)
EXTRA=$(comm -13 /tmp/pd_c_syms.txt /tmp/pd_r_syms.txt)
if [ -n "$MISSING" ]; then echo "!!! C symbols MISSING from Rust:"; echo "$MISSING"; FAIL=1
else echo "OK: 0 C symbols missing from Rust"; fi
if [ -n "$EXTRA" ]; then echo "NOTE: Rust-only exports:"; echo "$EXTRA"
else echo "OK: 0 Rust-only exports"; fi

# undefined non-libc symbols in the Rust .so
nm -D -u "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > /tmp/pd_r_undef.txt
NONLIBC=$(grep -v -E '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' /tmp/pd_r_undef.txt \
  | grep -v -x -E 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev|pthread_getspecific|sysconf|qsort|_exit|getpid|sigaction|sigaltstack|pthread_self|memrchr|memchr|strerror_r|__libc_start_main')
if [ -n "$NONLIBC" ]; then echo "!!! Rust .so has non-libc undefined symbols:"; echo "$NONLIBC"; FAIL=1
else echo "OK: 0 non-libc undefined symbols in Rust .so"; fi

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL PHASES PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
