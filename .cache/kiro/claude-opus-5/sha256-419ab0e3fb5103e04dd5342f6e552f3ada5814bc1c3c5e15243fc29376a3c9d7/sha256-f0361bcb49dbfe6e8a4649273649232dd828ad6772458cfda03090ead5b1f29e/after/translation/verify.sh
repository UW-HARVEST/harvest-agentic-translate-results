#!/usr/bin/env bash
# Phase D driver: rebuild both artifacts, diff exported symbols, and run the
# full differential suite under every cargo feature combination and profile.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
fail=0

echo "=============================================================="
echo "Phase D.1 — rebuild C and Rust shared objects"
echo "=============================================================="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
( cd "$CRATE" && timeout 600 cargo build >/dev/null 2>&1 \
  && timeout 600 cargo build --release >/dev/null 2>&1 ) \
  || { echo "Rust build FAILED"; exit 1; }

C_SO="$(find "$CBUILD" -maxdepth 1 -name 'lib*.so' | sort | tail -1)"
echo "C   .so: $C_SO"

echo
echo "=============================================================="
echo "Phase D.2 — exported-symbol parity (nm -D)"
echo "=============================================================="
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/pd_c.syms
for prof in debug release; do
  R_SO="$CRATE/target/$prof/libmd5_digest_lib.so"
  nm -D --defined-only "$R_SO" | awk '$(NF-1)=="T"{print $NF}' | sort -u > /tmp/pd_r.syms
  missing="$(comm -23 /tmp/pd_c.syms /tmp/pd_r.syms)"
  n_c=$(wc -l < /tmp/pd_c.syms)
  printf 'rust/%-7s : C exports %s, missing from Rust: %s\n' \
         "$prof" "$n_c" "$(printf '%s' "$missing" | grep -c . || true)"
  if [ -n "$missing" ]; then
    echo "  MISSING SYMBOLS:"; printf '    %s\n' $missing; fail=1
  fi
done

echo
echo "-- undefined non-libc symbols in the Rust .so --"
nm -D --undefined-only "$CRATE/target/release/libmd5_digest_lib.so" \
  | awk '{print $NF}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^__cxa_finalize$|^statx$|^gettid$|^__cxa_thread_atexit_impl$|^_Unwind_' \
  > /tmp/pd_undef.syms
n_undef=$(grep -c . /tmp/pd_undef.syms || true)
echo "count: $n_undef"
[ "$n_undef" -eq 0 ] || { cat /tmp/pd_undef.syms; fail=1; }

echo
echo "=============================================================="
echo "Phase D.3 — feature combinations"
echo "=============================================================="
# Enumerate features straight out of Cargo.toml rather than hardcoding them.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,"");split($0,a,"=");print a[1]}' \
      "$CRATE/Cargo.toml" | grep -v '^default$'
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the combination list: default, no-default, then each feature and the
# full powerset of the remaining ones (bounded; there are none here).
COMBOS=("" "--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((b=0; b<n; b++)); do
      (( mask & (1<<b) )) && sel="$sel,${FEATURES[$b]}"
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
    COMBOS+=("--features ${sel#,}")
  done
fi

echo "combination count: ${#COMBOS[@]}"
for combo in "${COMBOS[@]}"; do
  for prof in "" "--release"; do
    label="cargo test ${prof:-<dev>} ${combo:-<default features>}"
    ( cd "$CRATE" \
      && timeout 600 cargo build $prof $combo >/dev/null 2>&1 \
      && timeout 600 cargo test $prof $combo >/tmp/pd_test.log 2>&1 )
    rc=$?
    nbin="$(grep -c '^test result: ok' /tmp/pd_test.log || true)"
    npass="$(grep -oP '^test result: ok\. \K[0-9]+' /tmp/pd_test.log | awk '{s+=$1} END{print s+0}')"
    if [ $rc -eq 0 ]; then
      printf 'PASS  %-46s  (%s test binaries ok, %s tests passed)\n' "$label" "$nbin" "${npass:-?}"
    else
      printf 'FAIL  %-46s\n' "$label"
      grep -E '^test .* FAILED|^---- |^error' /tmp/pd_test.log | head -20
      fail=1
    fi
  done
done

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: ALL CHECKS PASSED"
else
  echo "PHASE D: FAILURES PRESENT"
fi
echo "=============================================================="
exit $fail
