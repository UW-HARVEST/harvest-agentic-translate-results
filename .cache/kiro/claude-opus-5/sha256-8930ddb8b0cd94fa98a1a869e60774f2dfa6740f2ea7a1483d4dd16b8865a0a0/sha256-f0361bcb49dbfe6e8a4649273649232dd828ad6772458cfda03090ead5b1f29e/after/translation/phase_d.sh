#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
R_SO="$CRATE/target/release/libcrc16_lib.so"
fail=0

echo "=== C   .so: $C_SO"
echo "=== RUST.so: $R_SO"

echo
echo "=== Symbol parity (nm -D --defined-only) ==="
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/pd_c.txt
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > /tmp/pd_r.txt
echo "C exports:    $(wc -l < /tmp/pd_c.txt)"
echo "Rust exports: $(wc -l < /tmp/pd_r.txt)"
missing="$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)"
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST:"; echo "$missing"; fail=1
else
  echo "symbol diff: EMPTY (0 missing)"
fi

echo
echo "=== Undefined non-libc/non-runtime symbols in Rust .so ==="
nm -D -u "$R_SO" | awk '{print $NF}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_|^statx$|^gettid$' \
  > /tmp/pd_undef.txt
if [ -s /tmp/pd_undef.txt ]; then
  echo "UNEXPECTED UNDEFINED:"; cat /tmp/pd_undef.txt; fail=1
else
  echo "0 unexpected undefined symbols"
fi

echo
echo "=== Feature combinations declared in Cargo.toml ==="
# Enumerate every feature (excluding "default") and build the full power set.
feats="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t"]/,"",a[1]); if(a[1]!="default") print a[1]}' "$CRATE/Cargo.toml")"
nfeat=0
[ -n "$feats" ] && nfeat=$(echo "$feats" | wc -l)
echo "declared features: ${nfeat} [${feats:-none}]"

run() { # run <label> <extra cargo args...>
  local label="$1"; shift
  echo "--- $label : cargo test --release $* ---"
  if timeout 600 cargo test --release "$@" --manifest-path "$CRATE/Cargo.toml" 2>&1 \
       | grep -E 'test result|^error' ; then :; fi
  local rc=${PIPESTATUS[0]}
  if [ "$rc" -ne 0 ]; then echo "  ==> $label FAILED (rc=$rc)"; fail=1; else echo "  ==> $label OK"; fi
}

combos=()
if [ "$nfeat" -eq 0 ]; then
  combos+=("default::")
  combos+=("no-default-features::--no-default-features")
  combos+=("all-features::--all-features")
else
  mapfile -t farr <<< "$feats"
  n=${#farr[@]}
  for ((m=0; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( m & (1<<i) )); then sel="${sel:+$sel,}${farr[i]}"; fi
    done
    combos+=("nodefault[${sel:-none}]::--no-default-features${sel:+ --features $sel}")
  done
  combos+=("default::")
  combos+=("all-features::--all-features")
fi

for c in "${combos[@]}"; do
  label="${c%%::*}"; args="${c#*::}"
  # shellcheck disable=SC2086
  run "$label" $args
done

echo
if [ "$fail" -eq 0 ]; then echo "PHASE D: ALL CHECKS PASSED"; else echo "PHASE D: FAILURES PRESENT"; fi
exit "$fail"
