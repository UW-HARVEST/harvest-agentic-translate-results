#!/usr/bin/env bash
# Enumerates every valid feature combination from translation/Cargo.toml and
# runs cargo check + cargo build + cargo test (debug and release) for each.
set -uo pipefail

cd "$(dirname "$0")/translation"

# Extract feature names from the [features] section, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "declared features (${n}): ${FEATURES[*]:-<none>}"

COMBOS=()
if [ "$n" -eq 0 ]; then
  COMBOS=("")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no-default-features>}"
  echo "=============================================================="
  echo "== $label"
  echo "=============================================================="
  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  for step in check build test; do
    for profile in "" --release; do
      log="/tmp/verify_${step}${profile//-/}_${combo//,/_}.log"
      if ! timeout 600 cargo "$step" "${args[@]}" $profile > "$log" 2>&1; then
        echo "FAIL: cargo $step ${args[*]} $profile (see $log)"
        tail -n 25 "$log"
        fail=1
      else
        echo "ok: cargo $step ${args[*]} $profile"
        [ "$step" = test ] && grep -E '^test result:' "$log" | sed 's/^/     /'
      fi
    done
  done
done

echo "=============================================================="
echo "== symbol comparison (C .so vs Rust .so)"
C_SO=$(ls ../c_src/build/*.so | head -1)
for R_SO in target/debug/libgaussian_kernel_lib.so target/release/libgaussian_kernel_lib.so; do
  [ -f "$R_SO" ] || continue
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "FAIL: $R_SO is missing symbols exported by $C_SO:"
    echo "$missing"
    fail=1
  else
    echo "ok: $R_SO exports every symbol from $(basename "$C_SO")"
  fi
done

[ "$fail" -eq 0 ] && echo "ALL CONFIGURATIONS PASS" || echo "FAILURES PRESENT"
exit "$fail"
