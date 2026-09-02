#!/bin/bash
# Enumerate every cargo feature combination declared in Cargo.toml and run
# `cargo check` (and, with -t, `cargo test`) for each one.
#
# Usage: tools/check_features.sh [-t]
set -uo pipefail
cd "$(dirname "$0")/.."

RUN_TESTS=0
[ "${1:-}" = "-t" ] && RUN_TESTS=1

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { infeat = 1; next }
    /^\[/           { infeat = 0 }
    infeat && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "features declared in Cargo.toml: $N ${FEATURES[*]:-(none)}"

if [ "$N" -eq 0 ]; then
  echo "==> only one feature combination exists (default == --no-default-features)"
  COMBOS=("")
else
  COMBOS=()
  TOTAL=$((1 << N))
  for ((mask = 0; mask < TOTAL; mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo "=================================================================="
  echo "==> combination: $label"
  if ! timeout 600 cargo check --release --no-default-features \
        ${combo:+--features "$combo"} > /tmp/feat_check.log 2>&1; then
    echo "    cargo check FAILED"; tail -n 20 /tmp/feat_check.log; FAIL=1; continue
  fi
  echo "    cargo check ok"
  if ! timeout 600 cargo build --release --no-default-features \
        ${combo:+--features "$combo"} > /tmp/feat_build.log 2>&1; then
    echo "    cargo build FAILED"; tail -n 20 /tmp/feat_build.log; FAIL=1; continue
  fi
  echo "    cargo build ok"
  # Symbol parity for this combination.
  nm -D --defined-only ../c_src/build/libpcre2.so | awk 'NF>=3{print $3}' | sort -u > /tmp/fc_c.txt
  nm -D --defined-only target/release/libpcre2.so | awk 'NF>=3{print $3}' | sort -u > /tmp/fc_r.txt
  miss=$(comm -23 /tmp/fc_c.txt /tmp/fc_r.txt | wc -l)
  extra=$(comm -13 /tmp/fc_c.txt /tmp/fc_r.txt | wc -l)
  echo "    symbols: missing=$miss extra=$extra"
  [ "$miss" -ne 0 ] && FAIL=1
  if [ "$RUN_TESTS" -eq 1 ]; then
    if ! timeout 1800 cargo test --release --no-default-features \
          ${combo:+--features "$combo"} -- --test-threads=1 > /tmp/feat_test.log 2>&1; then
      echo "    cargo test FAILED"; grep -E "^test |panicked" /tmp/feat_test.log | tail -n 20; FAIL=1; continue
    fi
    grep -E "test result" /tmp/feat_test.log | sed 's/^/    /'
  fi
done

echo "=================================================================="
if [ "$FAIL" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS OK"; else echo "SOME COMBINATIONS FAILED"; fi
exit $FAIL
