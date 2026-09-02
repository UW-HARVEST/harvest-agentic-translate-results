#!/usr/bin/env bash
# Phase D: enumerate every feature combination from Cargo.toml and run the
# full differential test suite under each. Derived mechanically from cargo
# metadata, not from assumptions.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 2>/dev/null |
  python3 -c "import json,sys; print('\n'.join(json.load(sys.stdin)['packages'][0]['features'].keys()))" |
  grep -v '^$'
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-<none>}"

# Build the combination list: always include the default build and the
# explicit no-default-features build; add the powerset of any real features.
COMBOS=("default:")
COMBOS+=("no-default:--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$joined:--no-default-features --features $joined")
  done
fi

echo "total combinations to verify: ${#COMBOS[@]}"
fail=0
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"
  flags="${entry#*:}"
  echo "=============================================================="
  echo "### combination: $name   (cargo $flags)"
  # shellcheck disable=SC2086
  if ! timeout 300 cargo build --release $flags >/tmp/fb_$name.log 2>&1; then
    echo "BUILD FAILED for $name"; tail -20 /tmp/fb_$name.log; fail=1; continue
  fi
  # symbol parity must hold for every combination
  nm -D --defined-only ../c_src/build/liblz4.so |
    awk '$2=="T"||$2=="B"||$2=="D"||$2=="R"{print $3}' | sort -u >/tmp/fc.txt
  nm -D --defined-only target/release/liblz4.so |
    awk '$2=="T"||$2=="B"||$2=="D"||$2=="R"{print $3}' | sort -u >/tmp/fr.txt
  miss=$(comm -23 /tmp/fc.txt /tmp/fr.txt | wc -l)
  extra=$(comm -13 /tmp/fc.txt /tmp/fr.txt | wc -l)
  echo "symbols: C=$(wc -l </tmp/fc.txt) Rust=$(wc -l </tmp/fr.txt) missing=$miss extra=$extra"
  [ "$miss" -eq 0 ] || { echo "SYMBOL PARITY FAILED for $name"; fail=1; }
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $flags >/tmp/ft_$name.log 2>&1; then
    echo "TESTS FAILED for $name"; grep -E "^test |test result|panicked" /tmp/ft_$name.log | tail -30; fail=1
  else
    grep -hE "test result" /tmp/ft_$name.log
  fi
done
echo "=============================================================="
[ "$fail" -eq 0 ] && echo "ALL COMBINATIONS PASSED" || echo "SOME COMBINATIONS FAILED"
exit $fail
