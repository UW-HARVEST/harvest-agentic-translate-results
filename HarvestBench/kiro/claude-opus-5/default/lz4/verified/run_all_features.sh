#!/usr/bin/env bash
# Phase D driver: enumerate every cargo feature combination from Cargo.toml and
# run the whole differential suite under each one. Also re-checks symbol parity
# for every combination, since features can change which symbols are exported.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=../c_src/build/liblz4.so
RS_SO=target/release/liblz4.so

# ---- discover features -------------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
if not m:
    raise SystemExit(0)
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != 'default':
        print(name)
PY
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the crate has exactly ONE configuration."
  COMBOS=("<default>")
else
  # power set of the declared features, plus the default build
  COMBOS=("<default>")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+="${FEATURES[$i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  echo "=============================================================="
  if [ "$combo" = "<default>" ]; then
    ARGS=()
    echo "CONFIGURATION: default features"
  elif [ -z "$combo" ]; then
    ARGS=(--no-default-features)
    echo "CONFIGURATION: --no-default-features"
  else
    ARGS=(--no-default-features --features "$combo")
    echo "CONFIGURATION: --no-default-features --features $combo"
  fi

  timeout 600 cargo build --release "${ARGS[@]}" >/dev/null 2>&1 || {
    echo "  BUILD FAILED"; fail=1; continue; }

  # symbol parity for this configuration
  cs=$(mktemp); rs=$(mktemp)
  nm -D --defined-only "$C_SO"  | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > "$cs"
  nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort -u > "$rs"
  missing=$(comm -23 "$cs" "$rs")
  extra=$(comm -13 "$cs" "$rs")
  echo "  symbols: C=$(wc -l < "$cs") Rust=$(wc -l < "$rs")"
  if [ -n "$missing" ]; then echo "  MISSING FROM RUST: $missing"; fail=1; fi
  if [ -n "$extra" ];   then echo "  EXTRA IN RUST:     $extra";   fail=1; fi
  rm -f "$cs" "$rs"

  for t in harness_selfcheck lz4_block lz4_stream repro_hc lz4hc xxhash \
           lz4frame_oneshot lz4frame_pipeline lz4frame_decode lz4file \
           errors_block errors_frame; do
    out=$(timeout 900 cargo test --release "${ARGS[@]}" --test "$t" -- --test-threads=1 2>&1)
    line=$(echo "$out" | grep -E '^test result' | tail -1)
    if echo "$line" | grep -q 'ok\.'; then
      echo "  PASS  $t  ${line#test result: }"
    else
      echo "  FAIL  $t"
      echo "$out" | grep -E 'panicked|assertion|signal|test result' | head -5 | sed 's/^/        /'
      fail=1
    fi
  done
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
